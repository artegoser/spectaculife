use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::{ImageSampler, ImageSamplerDescriptor},
    },
};
use rayon::prelude::*;

use crate::{
    cells::WorldCell,
    config::RenderConfig,
    grid::Grid,
    types::{Settings, State},
};

pub const CELL_WORLD_SIZE: f32 = 16.0;
const LIFE_TILE_SIZE: u32 = 16;
const ENERGY_TILE_SIZE: u32 = 16;

#[derive(Component)]
pub struct FarOverviewLayer;

#[derive(Clone)]
pub struct OverviewSourceAssets {
    pub life: Handle<Image>,
    pub organics: Handle<Image>,
    pub pollution: Handle<Image>,
    pub soil_energy: Handle<Image>,
    pub energy_directions: Handle<Image>,
}

#[derive(Clone, Copy, Default)]
struct LinearPremul {
    rgb: Vec3,
    a: f32,
}

impl LinearPremul {
    fn over(self, top: Self) -> Self {
        let keep_bottom = 1.0 - top.a;
        Self {
            rgb: top.rgb + self.rgb * keep_bottom,
            a: top.a + self.a * keep_bottom,
        }
    }
}

struct OverviewPalette {
    life: Vec<LinearPremul>,
    organics: Vec<LinearPremul>,
    pollution: Vec<LinearPremul>,
    soil_energy: Vec<LinearPremul>,
    energy_directions: Vec<LinearPremul>,
}

#[derive(Resource)]
pub struct OverviewRenderer {
    pub image: Handle<Image>,
    sources: OverviewSourceAssets,
    palette: Option<OverviewPalette>,
    level_a: Vec<LinearPremul>,
    level_b: Vec<LinearPremul>,
    encoded: Vec<u8>,
    last_overview_step: Option<usize>,
    last_overview_flags: u8,
    pub last_precise_step: Option<usize>,
}

impl OverviewRenderer {
    pub fn new(image: Handle<Image>, sources: OverviewSourceAssets, width: u32, height: u32) -> Self {
        let base_len = width as usize * height as usize;
        Self {
            image,
            sources,
            palette: None,
            level_a: vec![LinearPremul::default(); base_len],
            level_b: Vec::with_capacity(base_len / 4),
            encoded: Vec::with_capacity(mip_chain_bytes(width, height)),
            last_overview_step: None,
            last_overview_flags: 0,
            last_precise_step: None,
        }
    }

    pub fn needs_overview_rebuild(&self, state: &State) -> bool {
        self.last_overview_step != Some(state.simulation_step)
            || self.last_overview_flags != layer_flags(state)
    }

    pub fn rebuild_overview(
        &mut self,
        grid: &Grid<WorldCell>,
        settings: &Settings,
        config: &RenderConfig,
        state: &State,
        images: &mut Assets<Image>,
    ) -> bool {
        if self.palette.is_none() {
            self.palette = OverviewPalette::from_images(images, &self.sources);
        }
        let Some(palette) = self.palette.as_ref() else {
            return false;
        };

        let width = settings.w;
        let height = settings.h;
        let base_len = width as usize * height as usize;
        self.level_a.resize(base_len, LinearPremul::default());

        let flags = layer_flags(state);
        let soil_render_max = config.soil_energy_render_max.max(f32::EPSILON);
        self.level_a
            .par_iter_mut()
            .enumerate()
            .for_each(|(index, out)| {
                let cell = &grid.cells()[index];
                let mut color = LinearPremul::default();

                if flags & 1 != 0 {
                    color = color.over(palette_lookup(
                        &palette.organics,
                        cell.soil.organics as usize,
                    ));
                }
                if flags & 8 != 0 {
                    let soil_id = ((cell.soil.energy * 255.0 / soil_render_max) as usize).min(255);
                    color = color.over(palette_lookup(&palette.soil_energy, soil_id));
                }
                if flags & 2 != 0 {
                    let life_id = cell.life.texture_id(grid, index) as usize;
                    color = color.over(palette_lookup(&palette.life, life_id));
                }
                if flags & 16 != 0 {
                    let energy_id = cell.life.energy_directions_texture_id() as usize;
                    color = color.over(palette_lookup(&palette.energy_directions, energy_id));
                }
                if flags & 4 != 0 {
                    color = color.over(palette_lookup(
                        &palette.pollution,
                        cell.air.pollution as usize,
                    ));
                }

                *out = color;
            });

        self.encoded.clear();
        self.encoded.reserve(mip_chain_bytes(width, height));

        let mut current_width = width;
        let mut current_height = height;
        let mut current_is_a = true;

        loop {
            let current = if current_is_a {
                &self.level_a
            } else {
                &self.level_b
            };
            append_encoded_level(
                &mut self.encoded,
                &current[..current_width as usize * current_height as usize],
            );

            if current_width == 1 && current_height == 1 {
                break;
            }

            let next_width = (current_width / 2).max(1);
            let next_height = (current_height / 2).max(1);
            let next_len = next_width as usize * next_height as usize;

            if current_is_a {
                self.level_b.resize(next_len, LinearPremul::default());
                downsample_level(
                    &self.level_a,
                    current_width,
                    current_height,
                    &mut self.level_b,
                    next_width,
                    next_height,
                );
            } else {
                self.level_a.resize(next_len, LinearPremul::default());
                downsample_level(
                    &self.level_b,
                    current_width,
                    current_height,
                    &mut self.level_a,
                    next_width,
                    next_height,
                );
            }

            current_width = next_width;
            current_height = next_height;
            current_is_a = !current_is_a;
        }

        let Some(image) = images.get_mut(&self.image) else {
            return false;
        };
        std::mem::swap(&mut image.data, &mut self.encoded);

        self.last_overview_step = Some(state.simulation_step);
        self.last_overview_flags = flags;
        true
    }
}

impl OverviewPalette {
    fn from_images(images: &Assets<Image>, sources: &OverviewSourceAssets) -> Option<Self> {
        let life = images.get(&sources.life)?;
        let organics = images.get(&sources.organics)?;
        let pollution = images.get(&sources.pollution)?;
        let soil_energy = images.get(&sources.soil_energy)?;
        let energy_directions = images.get(&sources.energy_directions)?;

        Some(Self {
            life: average_tiles(life, LIFE_TILE_SIZE, LIFE_TILE_SIZE)?,
            organics: row_palette(organics)?,
            pollution: row_palette(pollution)?,
            soil_energy: row_palette(soil_energy)?,
            energy_directions: average_tiles(
                energy_directions,
                ENERGY_TILE_SIZE,
                ENERGY_TILE_SIZE,
            )?,
        })
    }
}

pub fn create_overview_image(width: u32, height: u32) -> Image {
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new(
        size,
        TextureDimension::D2,
        vec![0; width as usize * height as usize * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.mip_level_count = mip_level_count(width, height);
    image.data.resize(mip_chain_bytes(width, height), 0);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    image
}

pub fn overview_blend(camera_scale: f32, fade_start: f32, fade_end: f32) -> f32 {
    let start = fade_start.max(f32::EPSILON);
    let end = fade_end.max(start * 1.0001);
    if camera_scale <= start {
        return 0.0;
    }
    if camera_scale >= end {
        return 1.0;
    }

    // Camera zoom is exponential (wheel steps multiply scale by powers of two),
    // so interpolate in log-space. This makes the transition perceptually even
    // across zoom octaves instead of bunching most of the fade near one endpoint.
    let t = ((camera_scale / start).ln() / (end / start).ln()).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn layer_flags(state: &State) -> u8 {
    (state.organic_visible as u8)
        | ((state.life_visible as u8) << 1)
        | ((state.pollution_visible as u8) << 2)
        | ((state.soil_energy_visible as u8) << 3)
        | ((state.energy_directions_visible as u8) << 4)
}

fn mip_level_count(mut width: u32, mut height: u32) -> u32 {
    let mut count = 1;
    while width > 1 || height > 1 {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
        count += 1;
    }
    count
}

fn mip_chain_bytes(mut width: u32, mut height: u32) -> usize {
    let mut pixels = 0usize;
    loop {
        pixels += width as usize * height as usize;
        if width == 1 && height == 1 {
            break;
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    pixels * 4
}

fn downsample_level(
    source: &[LinearPremul],
    source_width: u32,
    source_height: u32,
    destination: &mut [LinearPremul],
    destination_width: u32,
    destination_height: u32,
) {
    destination
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, output)| {
            let dx = index as u32 % destination_width;
            let dy = index as u32 / destination_width;

            // Map each destination texel to its complete source footprint. For
            // power-of-two sizes this is exactly 2x2; for odd configured world
            // sizes this also includes the final row/column instead of dropping it.
            let sx0 = dx * source_width / destination_width;
            let sx1 = (((dx + 1) * source_width) / destination_width)
                .max(sx0 + 1)
                .min(source_width);
            let sy0 = dy * source_height / destination_height;
            let sy1 = (((dy + 1) * source_height) / destination_height)
                .max(sy0 + 1)
                .min(source_height);

            let mut sum_rgb = Vec3::ZERO;
            let mut sum_a = 0.0;
            let mut count = 0.0;
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let pixel = source[sy as usize * source_width as usize + sx as usize];
                    sum_rgb += pixel.rgb;
                    sum_a += pixel.a;
                    count += 1.0;
                }
            }

            *output = LinearPremul {
                rgb: sum_rgb / count,
                a: sum_a / count,
            };
        });
}

fn append_encoded_level(output: &mut Vec<u8>, pixels: &[LinearPremul]) {
    let start = output.len();
    output.resize(start + pixels.len() * 4, 0);
    output[start..]
        .par_chunks_mut(4)
        .zip(pixels.par_iter())
        .for_each(|(bytes, pixel)| {
            let rgba = encode_pixel(*pixel);
            bytes.copy_from_slice(&rgba);
        });
}

fn palette_lookup(palette: &[LinearPremul], index: usize) -> LinearPremul {
    palette
        .get(index)
        .copied()
        .or_else(|| palette.last().copied())
        .unwrap_or_default()
}

fn row_palette(image: &Image) -> Option<Vec<LinearPremul>> {
    if image.height() < 1 {
        return None;
    }
    (0..image.width())
        .map(|x| read_pixel(image, x, 0))
        .collect()
}

fn average_tiles(image: &Image, tile_width: u32, tile_height: u32) -> Option<Vec<LinearPremul>> {
    if tile_width == 0 || tile_height == 0 || image.height() < tile_height {
        return None;
    }
    let tile_count = image.width() / tile_width;
    let mut result = Vec::with_capacity(tile_count as usize);

    for tile in 0..tile_count {
        let mut sum_rgb = Vec3::ZERO;
        let mut sum_a = 0.0;
        let mut count = 0.0;
        for y in 0..tile_height {
            for x in 0..tile_width {
                let pixel = read_pixel(image, tile * tile_width + x, y)?;
                sum_rgb += pixel.rgb;
                sum_a += pixel.a;
                count += 1.0;
            }
        }
        result.push(LinearPremul {
            rgb: sum_rgb / count,
            a: sum_a / count,
        });
    }

    Some(result)
}

fn read_pixel(image: &Image, x: u32, y: u32) -> Option<LinearPremul> {
    if x >= image.width() || y >= image.height() {
        return None;
    }
    let srgb = match image.texture_descriptor.format {
        TextureFormat::Rgba8UnormSrgb => true,
        TextureFormat::Rgba8Unorm => false,
        _ => return None,
    };
    let index = (y as usize * image.width() as usize + x as usize) * 4;
    let bytes = image.data.get(index..index + 4)?;
    let alpha = bytes[3] as f32 / 255.0;
    let decode = |v: u8| {
        let c = v as f32 / 255.0;
        if srgb {
            srgb_to_linear(c)
        } else {
            c
        }
    };
    Some(LinearPremul {
        rgb: Vec3::new(decode(bytes[0]), decode(bytes[1]), decode(bytes[2])) * alpha,
        a: alpha,
    })
}

fn encode_pixel(pixel: LinearPremul) -> [u8; 4] {
    let alpha = pixel.a.clamp(0.0, 1.0);
    let straight = if alpha > 1.0e-6 {
        (pixel.rgb / alpha).clamp(Vec3::ZERO, Vec3::ONE)
    } else {
        Vec3::ZERO
    };
    let encode = |linear: f32| -> u8 {
        (linear_to_srgb(linear.clamp(0.0, 1.0)) * 255.0 + 0.5) as u8
    };
    [
        encode(straight.x),
        encode(straight.y),
        encode(straight.z),
        (alpha * 255.0 + 0.5) as u8,
    ]
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chain_for_512_is_complete() {
        assert_eq!(mip_level_count(512, 512), 10);
        assert_eq!(mip_chain_bytes(512, 512), 1_398_100);
    }

    #[test]
    fn odd_sizes_reach_one_by_one() {
        assert_eq!(mip_level_count(513, 257), 10);
        assert!(mip_chain_bytes(513, 257) > 513 * 257 * 4);
    }

    #[test]
    fn overview_fade_is_smooth_in_zoom_octaves() {
        assert_eq!(overview_blend(4.0, 4.0, 16.0), 0.0);
        assert!((overview_blend(8.0, 4.0, 16.0) - 0.5).abs() < 1.0e-6);
        assert_eq!(overview_blend(16.0, 4.0, 16.0), 1.0);
    }
}
