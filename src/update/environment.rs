use rayon::prelude::*;

use crate::{
    cells::WorldCell,
    config::{EnvironmentConfig, WindConfig},
    grid::Grid,
};

/// Build a coherent, periodic target wind field for one weather epoch.
///
/// The expensive value-noise sampling only runs once per `gust_period_ticks`.
/// The active wind field approaches this target gradually inside
/// `compute_environment`, so gust boundaries never cause an instantaneous jump.
pub fn refresh_wind_targets(
    grid: &Grid<WorldCell>,
    target_x: &mut [f32],
    target_y: &mut [f32],
    epoch: u64,
    config: &WindConfig,
) {
    debug_assert_eq!(target_x.len(), grid.len());
    debug_assert_eq!(target_y.len(), grid.len());

    if !config.enabled {
        target_x.par_iter_mut().for_each(|value| *value = 0.0);
        target_y.par_iter_mut().for_each(|value| *value = 0.0);
        return;
    }

    let width = grid.width as usize;
    let height = grid.height as usize;
    let scale = config.spatial_scale_cells.max(2) as f32;
    let lattice_w = ((width as f32 / scale).ceil() as usize).max(2);
    let lattice_h = ((height as f32 / scale).ceil() as usize).max(2);

    // Generate the random lattice once per weather epoch.  The previous
    // implementation hashed all four lattice corners independently for every
    // world cell and for both vector components. On a 512x512 world that meant
    // ~2.1 million 64-bit hash evaluations in one simulation tick even though
    // the default 56-cell spatial scale contains only about a 10x10 lattice.
    let lattice_x = build_noise_lattice(lattice_w, lattice_h, epoch, config.seed);
    let lattice_y = build_noise_lattice(
        lattice_w,
        lattice_h,
        epoch,
        config.seed ^ 0x9E37_79B9_7F4A_7C15,
    );

    // X interpolation state is shared by every row and therefore also worth
    // precomputing once.  Y has one small tuple per row.
    let x_samples: Vec<(usize, usize, f32)> = (0..width)
        .map(|x| {
            let gx = x as f32 * lattice_w as f32 / width as f32;
            let x0 = gx.floor() as usize % lattice_w;
            let x1 = (x0 + 1) % lattice_w;
            (x0, x1, smoothstep(gx - gx.floor()))
        })
        .collect();
    let y_samples: Vec<(usize, usize, f32)> = (0..height)
        .map(|y| {
            let gy = y as f32 * lattice_h as f32 / height as f32;
            let y0 = gy.floor() as usize % lattice_h;
            let y1 = (y0 + 1) % lattice_h;
            (y0, y1, smoothstep(gy - gy.floor()))
        })
        .collect();

    target_x
        .par_chunks_mut(width)
        .zip(target_y.par_chunks_mut(width))
        .enumerate()
        .for_each(|(y, (row_x, row_y))| {
            let (y0, y1, ty) = y_samples[y];
            let row0 = y0 * lattice_w;
            let row1 = y1 * lattice_w;

            for x in 0..width {
                let (x0, x1, tx) = x_samples[x];
                let gust_x = bilerp_lattice(&lattice_x, row0, row1, x0, x1, tx, ty);
                let gust_y = bilerp_lattice(&lattice_y, row0, row1, x0, x1, tx, ty);

                let mut vx = config.base_x + gust_x * config.gust_strength;
                let mut vy = config.base_y + gust_y * config.gust_strength;

                // Advection is deliberately limited to the local neighborhood.
                let speed_sq = vx * vx + vy * vy;
                if speed_sq > 1.0 {
                    let inv_speed = speed_sq.sqrt().recip();
                    vx *= inv_speed;
                    vy *= inv_speed;
                }

                row_x[x] = vx;
                row_y[x] = vy;
            }
        });
}

#[inline]
fn build_noise_lattice(
    width: usize,
    height: usize,
    epoch: u64,
    seed: u64,
) -> Vec<f32> {
    let mut lattice = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            lattice.push(lattice_noise(x, y, epoch, seed));
        }
    }
    lattice
}

#[inline]
fn bilerp_lattice(
    lattice: &[f32],
    row0: usize,
    row1: usize,
    x0: usize,
    x1: usize,
    tx: f32,
    ty: f32,
) -> f32 {
    let n00 = lattice[row0 + x0];
    let n10 = lattice[row0 + x1];
    let n01 = lattice[row1 + x0];
    let n11 = lattice[row1 + x1];
    let top = n00 + (n10 - n00) * tx;
    let bottom = n01 + (n11 - n01) * tx;
    top + (bottom - top) * ty
}

/// Diffuse soil energy and transport airborne pollution from the same immutable
/// world state.
///
/// Pollution uses a semi-Lagrangian one-cell advection step through a persistent
/// wind field, followed by weak local diffusion and atmospheric decay.  The
/// source position never moves by more than one cell per tick, so a gust cannot
/// teleport a plume across the map in one simulation step.
pub fn compute_environment(
    grid: &Grid<WorldCell>,
    next_soil_energy: &mut [f32],
    next_pollution: &mut [f32],
    wind_x: &mut [f32],
    wind_y: &mut [f32],
    target_wind_x: &[f32],
    target_wind_y: &[f32],
    config: &EnvironmentConfig,
) {
    let width = grid.width as usize;
    let height = grid.height as usize;
    debug_assert_eq!(next_soil_energy.len(), grid.len());
    debug_assert_eq!(next_pollution.len(), grid.len());
    debug_assert_eq!(wind_x.len(), grid.len());
    debug_assert_eq!(wind_y.len(), grid.len());
    debug_assert_eq!(target_wind_x.len(), grid.len());
    debug_assert_eq!(target_wind_y.len(), grid.len());

    let cells = grid.cells();
    let soil_diffusion = config.soil_diffusion.clamp(0.0, 1.0);
    let air_diffusion = config.air_diffusion.clamp(0.0, 1.0);
    let pollution_decay = config.pollution_decay.clamp(0.0, 1.0);
    let response = config.wind.response_per_tick.clamp(0.0, 1.0);
    let advection = config
        .wind
        .advection_cells_per_tick
        .clamp(0.0, 1.0);
    let wind_enabled = config.wind.enabled;

    next_soil_energy
        .par_chunks_mut(width)
        .zip(next_pollution.par_chunks_mut(width))
        .zip(wind_x.par_chunks_mut(width))
        .zip(wind_y.par_chunks_mut(width))
        .zip(target_wind_x.par_chunks(width))
        .zip(target_wind_y.par_chunks(width))
        .enumerate()
        .for_each(
            |(y, (((((soil_row, pollution_row), wind_x_row), wind_y_row), target_x_row), target_y_row))| {
                let yu = if y == 0 { height - 1 } else { y - 1 };
                let yd = if y + 1 == height { 0 } else { y + 1 };
                let row = y * width;
                let row_up = yu * width;
                let row_down = yd * width;

                for x in 0..width {
                    let xl = if x == 0 { width - 1 } else { x - 1 };
                    let xr = if x + 1 == width { 0 } else { x + 1 };
                    let center_idx = row + x;

                    let neighbor_indices = [
                        row_up + xl,
                        row_up + x,
                        row_up + xr,
                        row + xl,
                        row + xr,
                        row_down + xl,
                        row_down + x,
                        row_down + xr,
                    ];

                    let mut soil_total = 0.0_f32;
                    let mut pollution_total = 0.0_f32;
                    for neighbor in neighbor_indices {
                        let cell = &cells[neighbor];
                        soil_total += cell.soil.energy;
                        pollution_total += cell.air.pollution as f32;
                    }

                    let center = &cells[center_idx];
                    let soil_neighbor_avg = soil_total * 0.125;
                    soil_row[x] = (center.soil.energy
                        + (soil_neighbor_avg - center.soil.energy) * soil_diffusion)
                        .max(0.0);

                    let (wx, wy) = if wind_enabled {
                        let wx = wind_x_row[x]
                            + (target_x_row[x] - wind_x_row[x]) * response;
                        let wy = wind_y_row[x]
                            + (target_y_row[x] - wind_y_row[x]) * response;
                        wind_x_row[x] = wx;
                        wind_y_row[x] = wy;
                        (wx, wy)
                    } else {
                        wind_x_row[x] = 0.0;
                        wind_y_row[x] = 0.0;
                        (0.0, 0.0)
                    };

                    let center_pollution = center.air.pollution as f32;
                    let advected = if wind_enabled && advection > 0.0 {
                        sample_pollution_toroidal(
                            cells,
                            width,
                            height,
                            x as f32 - wx * advection,
                            y as f32 - wy * advection,
                        )
                    } else {
                        center_pollution
                    };
                    let pollution_neighbor_avg = pollution_total * 0.125;
                    let mixed = advected * (1.0 - air_diffusion)
                        + pollution_neighbor_avg * air_diffusion;
                    pollution_row[x] = (mixed * (1.0 - pollution_decay)).clamp(0.0, 255.0);
                }
            },
        );
}

#[inline]
fn sample_pollution_toroidal(
    cells: &[WorldCell],
    width: usize,
    height: usize,
    x: f32,
    y: f32,
) -> f32 {
    // Do not wrap in f32 first. Close to the negative map boundary, floating-point
    // rounding can make `rem_euclid(width as f32)` equal exactly `width`, which
    // turns into an out-of-bounds integer index after `floor()`. Compute the
    // interpolation fraction in float, but wrap the integer lattice coordinate.
    let xf = x.floor();
    let yf = y.floor();
    let x0 = (xf as i64).rem_euclid(width as i64) as usize;
    let y0 = (yf as i64).rem_euclid(height as i64) as usize;
    let x1 = (x0 + 1) % width;
    let y1 = (y0 + 1) % height;
    let tx = (x - xf).clamp(0.0, 1.0);
    let ty = (y - yf).clamp(0.0, 1.0);

    let p00 = cells[y0 * width + x0].air.pollution as f32;
    let p10 = cells[y0 * width + x1].air.pollution as f32;
    let p01 = cells[y1 * width + x0].air.pollution as f32;
    let p11 = cells[y1 * width + x1].air.pollution as f32;

    let top = p00 + (p10 - p00) * tx;
    let bottom = p01 + (p11 - p01) * tx;
    top + (bottom - top) * ty
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn lattice_noise(x: usize, y: usize, epoch: u64, seed: u64) -> f32 {
    let mut value = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ epoch.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;

    let unit = ((value >> 40) & 0x00FF_FFFF) as f32 / 16_777_215.0;
    unit * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_wind(enabled: bool) -> EnvironmentConfig {
        EnvironmentConfig {
            soil_diffusion: 1.0,
            air_diffusion: 1.0,
            pollution_decay: 0.0,
            wind: WindConfig {
                enabled,
                base_x: 0.0,
                base_y: 0.0,
                gust_strength: 0.0,
                spatial_scale_cells: 8,
                gust_period_ticks: 64,
                response_per_tick: 1.0,
                advection_cells_per_tick: 1.0,
                seed: 1,
            },
        }
    }

    #[test]
    fn environment_does_not_skip_cells_in_one_tick() {
        let mut grid = Grid::<WorldCell>::new(7, 7);
        grid.uget_mut(3, 3).soil.energy = 100.0;
        grid.uget_mut(3, 3).air.pollution = 100;
        let mut next_soil = vec![0.0; grid.len()];
        let mut next_pollution = vec![0.0; grid.len()];
        let mut wind_x = vec![0.0; grid.len()];
        let mut wind_y = vec![0.0; grid.len()];
        let target_x = vec![0.0; grid.len()];
        let target_y = vec![0.0; grid.len()];

        compute_environment(
            &grid,
            &mut next_soil,
            &mut next_pollution,
            &mut wind_x,
            &mut wind_y,
            &target_x,
            &target_y,
            &config_with_wind(false),
        );

        let neighbor = 3 * 7 + 4;
        let two_away = 3 * 7 + 5;
        assert!(next_soil[neighbor] > 0.0);
        assert_eq!(next_soil[two_away], 0.0);
        assert!(next_pollution[neighbor] > 0.0);
        assert_eq!(next_pollution[two_away], 0.0);
    }

    #[test]
    fn wind_advects_pollution_only_one_cell_per_tick() {
        let mut grid = Grid::<WorldCell>::new(7, 7);
        grid.uget_mut(3, 3).air.pollution = 100;
        let mut next_soil = vec![0.0; grid.len()];
        let mut next_pollution = vec![0.0; grid.len()];
        let mut wind_x = vec![1.0; grid.len()];
        let mut wind_y = vec![0.0; grid.len()];
        let target_x = vec![1.0; grid.len()];
        let target_y = vec![0.0; grid.len()];
        let mut config = config_with_wind(true);
        config.soil_diffusion = 0.0;
        config.air_diffusion = 0.0;

        compute_environment(
            &grid,
            &mut next_soil,
            &mut next_pollution,
            &mut wind_x,
            &mut wind_y,
            &target_x,
            &target_y,
            &config,
        );

        let downwind = 3 * 7 + 4;
        let two_away = 3 * 7 + 5;
        assert!(next_pollution[downwind] > 99.0);
        assert_eq!(next_pollution[two_away], 0.0);
    }


    #[test]
    fn toroidal_sampling_never_indexes_past_float_boundary() {
        let mut grid = Grid::<WorldCell>::new(512, 512);
        grid.uget_mut(94, 0).air.pollution = 123;

        // A tiny negative coordinate is the problematic case: wrapping it in f32
        // can round to exactly 512.0 on a 512-wide/512-high map.
        let sampled = sample_pollution_toroidal(
            grid.cells(),
            512,
            512,
            94.0,
            -f32::EPSILON,
        );
        assert!(sampled.is_finite());
    }

    #[test]
    fn generated_gust_field_is_spatially_non_uniform() {
        let grid = Grid::<WorldCell>::new(32, 32);
        let mut target_x = vec![0.0; grid.len()];
        let mut target_y = vec![0.0; grid.len()];
        let mut config = config_with_wind(true).wind;
        config.gust_strength = 1.0;
        config.spatial_scale_cells = 8;

        refresh_wind_targets(&grid, &mut target_x, &mut target_y, 7, &config);

        assert!(target_x.iter().any(|value| (*value - target_x[0]).abs() > 0.01));
        assert!(target_y.iter().any(|value| (*value - target_y[0]).abs() > 0.01));
    }
}
