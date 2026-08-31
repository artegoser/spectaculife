use crate::cells::{
    life_cell::{
        genome::{Genome, GenomePool},
        AliveCell, EnergyDirections, LifeCell, LifeType::*,
    },
    WorldCell,
};
use crate::config::{RenderConfig, SimulationConfig};
use crate::grid::Grid;
use crate::types::{Settings, State};
use crate::update::{update_simulation_step, SimulationBuffers};
use crate::utils::get_map;
use super::overview::{
    create_overview_image, overview_blend, FarOverviewLayer, OverviewRenderer, OverviewSourceAssets,
    CELL_WORLD_SIZE,
};
use bevy::math::{uvec2, vec2, vec3};
use bevy::prelude::*;
use bevy_fast_tilemap::prelude::*;
use rand::Rng;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    mpsc, Arc, Mutex, Weak,
};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct WorldPlugin;

#[derive(Component)]
pub struct OrganicsLayer;
#[derive(Component)]
pub struct LifeLayer;
#[derive(Component)]
pub struct PollutionLayer;
#[derive(Component)]
pub struct SoilEnergyLayer;
#[derive(Component)]
pub struct EnergyDirectionsLayer;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let (w, h) = {
            let config = app.world().resource::<SimulationConfig>();
            (config.world.width, config.world.height)
        };

        app.add_plugins(FastTileMapPlugin::default())
            .add_systems(Startup, startup)
            .add_systems(Update, (receive_and_render, sync_render_lod_visibility).chain())
            .insert_resource(Grid::<WorldCell>::default())
            .insert_resource(Settings { w, h })
            .insert_resource(State::default());
    }
}

#[derive(Resource)]
pub struct SimulationWorker {
    latest_snapshot: Arc<Mutex<Option<SimSnapshot>>>,
    command_sender: Mutex<mpsc::Sender<SimCommand>>,
    paused: Arc<AtomicBool>,
    step_requested: Arc<AtomicBool>,
    avg_tick_ns: Arc<AtomicU64>,
    worker_alive: Arc<AtomicBool>,
    debug_phase: Arc<AtomicU8>,
    active_tick: Arc<AtomicU64>,
    completed_tick: Arc<AtomicU64>,
}

impl SimulationWorker {
    pub fn request_step(&self) {
        self.step_requested.store(true, Ordering::Relaxed);
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn reinitialize(&self) {
        let _ = self.command_sender.lock().unwrap().send(SimCommand::Reset);
    }

    pub fn average_tick_ms(&self) -> f64 {
        self.avg_tick_ns.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }

    pub fn ticks_per_second(&self) -> f64 {
        let ns = self.avg_tick_ns.load(Ordering::Relaxed);
        if ns == 0 {
            0.0
        } else {
            1_000_000_000.0 / ns as f64
        }
    }

    pub fn debug_status(&self) -> (bool, &'static str, u64, u64) {
        let phase = match self.debug_phase.load(Ordering::Relaxed) {
            1 => "wind-refresh",
            2 => "environment",
            3 => "life",
            4 => "publish",
            5 => "paused",
            _ => "idle",
        };
        (
            self.worker_alive.load(Ordering::Relaxed),
            phase,
            self.active_tick.load(Ordering::Relaxed),
            self.completed_tick.load(Ordering::Relaxed),
        )
    }
}

struct SimSnapshot {
    grid: Grid<WorldCell>,
    step: usize,
}

enum SimCommand {
    Reset,
}

fn spawn_sim_thread(
    grid: Grid<WorldCell>,
    genomes: GenomePool,
    settings: Settings,
    config: SimulationConfig,
    paused: Arc<AtomicBool>,
    step_requested: Arc<AtomicBool>,
    next_organism_id: u64,
) -> (
    Arc<Mutex<Option<SimSnapshot>>>,
    mpsc::Sender<SimCommand>,
    Arc<AtomicU64>,
    Arc<AtomicBool>,
    Arc<AtomicU8>,
    Arc<AtomicU64>,
    Arc<AtomicU64>,
) {
    let latest_snapshot = Arc::new(Mutex::new(None));
    let snapshot_slot = Arc::downgrade(&latest_snapshot);
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let avg_tick_ns = Arc::new(AtomicU64::new(0));
    let thread_avg_tick_ns = avg_tick_ns.clone();
    let worker_alive = Arc::new(AtomicBool::new(true));
    let thread_worker_alive = worker_alive.clone();
    let debug_phase = Arc::new(AtomicU8::new(0));
    let thread_debug_phase = debug_phase.clone();
    let active_tick = Arc::new(AtomicU64::new(0));
    let thread_active_tick = active_tick.clone();
    let completed_tick = Arc::new(AtomicU64::new(0));
    let thread_completed_tick = completed_tick.clone();

    thread::spawn(move || {
        struct AliveGuard(Arc<AtomicBool>);
        impl Drop for AliveGuard {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Relaxed);
            }
        }
        let _alive_guard = AliveGuard(thread_worker_alive);
        let mut grid = grid;
        let mut genomes = genomes;
        let mut step: usize = 0;
        let mut sim_state = State::default();
        sim_state.initialized = true;
        sim_state.next_organism_id = next_organism_id;
        let mut simulation_buffers = SimulationBuffers::new(&grid);

        loop {
            let mut reset_happened = false;
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    SimCommand::Reset => {
                        grid = Grid::<WorldCell>::new(settings.w, settings.h);
                        genomes = GenomePool::new();
                        sim_state.next_organism_id =
                            populate_grid(&mut grid, &mut genomes, &settings, &config);
                        step = 0;
                        sim_state.simulation_step = 0;
                        simulation_buffers = SimulationBuffers::new(&grid);
                        thread_avg_tick_ns.store(0, Ordering::Relaxed);
                        thread_active_tick.store(0, Ordering::Relaxed);
                        thread_completed_tick.store(0, Ordering::Relaxed);
                        thread_debug_phase.store(0, Ordering::Relaxed);
                        reset_happened = true;
                    }
                }
            }

            if reset_happened && !publish_snapshot(&snapshot_slot, &grid, step, true) {
                break;
            }

            let do_step = step_requested.swap(false, Ordering::Relaxed);
            let is_paused = paused.load(Ordering::Relaxed);
            if is_paused && !do_step {
                thread_debug_phase.store(5, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(1));
                continue;
            }

            let tick_started = Instant::now();
            let running_tick = step.saturating_add(1) as u64;
            thread_active_tick.store(running_tick, Ordering::Relaxed);
            thread_debug_phase.store(1, Ordering::Relaxed);
            update_simulation_step(
                &mut sim_state,
                &mut grid,
                &mut genomes,
                &mut simulation_buffers,
                &config,
                &thread_debug_phase,
            );

            step += 1;
            sim_state.simulation_step = step;
            thread_debug_phase.store(4, Ordering::Relaxed);

            if !publish_snapshot(&snapshot_slot, &grid, step, false) {
                break;
            }

            thread_completed_tick.store(step as u64, Ordering::Relaxed);
            thread_debug_phase.store(0, Ordering::Relaxed);

            let elapsed_ns = tick_started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
            let previous = thread_avg_tick_ns.load(Ordering::Relaxed);
            let smoothed = if previous == 0 {
                elapsed_ns
            } else {
                previous.saturating_mul(15) / 16 + elapsed_ns / 16
            };
            thread_avg_tick_ns.store(smoothed.max(1), Ordering::Relaxed);
        }
    });

    (
        latest_snapshot,
        cmd_tx,
        avg_tick_ns,
        worker_alive,
        debug_phase,
        active_tick,
        completed_tick,
    )
}

fn publish_snapshot(
    snapshot_slot: &Weak<Mutex<Option<SimSnapshot>>>,
    grid: &Grid<WorldCell>,
    step: usize,
    replace_pending: bool,
) -> bool {
    let Some(snapshot_slot) = snapshot_slot.upgrade() else {
        return false;
    };

    let mut slot = snapshot_slot.lock().unwrap();
    // The renderer is a one-slot mailbox. If it has not consumed the previous
    // frame yet, do not clone another full 512x512 world just to overwrite it.
    // Reset is the exception: it replaces a stale pending frame immediately.
    if replace_pending || slot.is_none() {
        *slot = Some(SimSnapshot {
            grid: grid.clone(),
            step,
        });
    }
    true
}

fn populate_grid(
    grid: &mut Grid<WorldCell>,
    genomes: &mut GenomePool,
    settings: &Settings,
    config: &SimulationConfig,
) -> u64 {
    let mut rng = rand::thread_rng();
    let mut next_organism_id = 1_u64;

    for x in 0..settings.w {
        for y in 0..settings.h {
            let cell = grid.get_mut(x as i64, y as i64);
            *cell = WorldCell::default();

            cell.soil.organics = config.world.initial_organics.sample(&mut rng);
            cell.soil.energy = config.world.initial_soil_energy.sample(&mut rng);
            cell.air.pollution = config.world.initial_pollution.sample(&mut rng);

            if x % config.world.organism_spacing == 0
                && y % config.world.organism_spacing == 0
            {
                let genome = Genome::random(&mut rng, &config.genetics);
                let handle = genomes.alloc(genome);
                let organism_id = next_organism_id;
                next_organism_id = next_organism_id.saturating_add(1);

                let mut founder = AliveCell::new(
                    Stem(handle),
                    organism_id,
                    config.world.initial_stem_energy,
                    EnergyDirections::default(),
                    None,
                    config.world.initial_stem_lifespan,
                );
                founder.heading = rng.gen();
                cell.life = LifeCell::Alive(founder);
            }
        }
    }

    next_organism_id
}

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<Map>>,
    mut images: ResMut<Assets<Image>>,
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    config: Res<SimulationConfig>,
    mut state: ResMut<State>,
) {
    commands.spawn(Camera2dBundle::default());

    *world = Grid::<WorldCell>::new(settings.w, settings.h);

    let mut genomes = GenomePool::new();
    let next_organism_id = populate_grid(&mut world, &mut genomes, &settings, &config);
    state.initialized = true;
    state.next_organism_id = next_organism_id;

    let paused = Arc::new(AtomicBool::new(false));
    let step_requested = Arc::new(AtomicBool::new(false));

    let (
        latest_snapshot,
        cmd_tx,
        avg_tick_ns,
        worker_alive,
        debug_phase,
        active_tick,
        completed_tick,
    ) = spawn_sim_thread(
        world.clone(),
        genomes,
        *settings,
        (*config).clone(),
        paused.clone(),
        step_requested.clone(),
        next_organism_id,
    );

    commands.insert_resource(SimulationWorker {
        latest_snapshot,
        command_sender: Mutex::new(cmd_tx),
        paused,
        step_requested,
        avg_tick_ns,
        worker_alive,
        debug_phase,
        active_tick,
        completed_tick,
    });

    // The detailed renderer stays on bevy_fast_tilemap: each layer is already a single
    // GPU quad. The life atlas is padded to a power-of-two width to avoid the precision
    // seams that become visible at awkward zoom factors.
    let life_texture = asset_server.load("life_pot.png");
    let organics_texture = asset_server.load("organics.png");
    let pollution_texture = asset_server.load("pollution.png");
    let soil_energy_texture = asset_server.load("soil_energy.png");
    let energy_directions_texture = asset_server.load("energy_directions.png");

    let cell_map = Map::builder(
        uvec2(settings.w, settings.h),
        life_texture.clone(),
        vec2(16., 16.),
    )
    .build();
    let organics_map = Map::builder(
        uvec2(settings.w, settings.h),
        organics_texture.clone(),
        vec2(1., 1.),
    )
    .build();
    let pollution_map = Map::builder(
        uvec2(settings.w, settings.h),
        pollution_texture.clone(),
        vec2(1., 1.),
    )
    .build();
    let soil_energy_map = Map::builder(
        uvec2(settings.w, settings.h),
        soil_energy_texture.clone(),
        vec2(1., 1.),
    )
    .build();
    let energy_directions_map = Map::builder(
        uvec2(settings.w, settings.h),
        energy_directions_texture.clone(),
        vec2(16., 16.),
    )
    .build();

    // At large zoom-out a tile-index renderer fundamentally undersamples the map: a
    // screen pixel covers many cells but the shader still picks one cell. The far LOD
    // is therefore a real mipmapped image of the composed world, not mipmaps of the
    // atlas. This removes shimmer/moire and makes density visible at any distance.
    let overview_image = images.add(create_overview_image(settings.w, settings.h));
    commands.spawn((
        SpriteBundle {
            texture: overview_image.clone(),
            sprite: Sprite {
                custom_size: Some(vec2(
                    settings.w as f32 * CELL_WORLD_SIZE,
                    settings.h as f32 * CELL_WORLD_SIZE,
                )),
                ..default()
            },
            transform: Transform::from_translation(vec3(0.0, 0.0, 10.0)),
            visibility: Visibility::Hidden,
            ..default()
        },
        FarOverviewLayer,
    ));
    commands.insert_resource(OverviewRenderer::new(
        overview_image,
        OverviewSourceAssets {
            life: life_texture,
            organics: organics_texture,
            pollution: pollution_texture,
            soil_energy: soil_energy_texture,
            energy_directions: energy_directions_texture,
        },
        settings.w,
        settings.h,
    ));

    commands.spawn((
        MapBundleManaged {
            material: materials.add(organics_map),
            transform: Transform::default().with_scale(vec3(16., 16., 1.)),
            ..default()
        },
        OrganicsLayer,
    ));
    commands.spawn((
        MapBundleManaged {
            material: materials.add(cell_map),
            transform: Transform::default().with_translation(vec3(0., 0., 2.)),
            ..default()
        },
        LifeLayer,
    ));
    commands.spawn((
        MapBundleManaged {
            material: materials.add(pollution_map),
            transform: Transform::default()
                .with_translation(vec3(0., 0., 4.))
                .with_scale(vec3(16., 16., 1.)),
            ..default()
        },
        PollutionLayer,
    ));
    commands.spawn((
        MapBundleManaged {
            material: materials.add(soil_energy_map),
            transform: Transform::default()
                .with_translation(vec3(0., 0., 1.))
                .with_scale(vec3(16., 16., 1.)),
            ..default()
        },
        SoilEnergyLayer,
    ));
    commands.spawn((
        MapBundleManaged {
            material: materials.add(energy_directions_map),
            transform: Transform::default().with_translation(vec3(0., 0., 3.)),
            ..default()
        },
        EnergyDirectionsLayer,
    ));
}

fn receive_and_render(
    mut map_materials: ResMut<Assets<Map>>,
    mut images: ResMut<Assets<Image>>,
    mut overview: ResMut<OverviewRenderer>,
    organics_handle: Query<&Handle<Map>, With<OrganicsLayer>>,
    life_handle: Query<&Handle<Map>, With<LifeLayer>>,
    pollution_handle: Query<&Handle<Map>, With<PollutionLayer>>,
    soil_energy_handle: Query<&Handle<Map>, With<SoilEnergyLayer>>,
    energy_directions_handle: Query<&Handle<Map>, With<EnergyDirectionsLayer>>,
    camera: Query<&Transform, With<Camera>>,
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    render_config: Res<RenderConfig>,
    mut state: ResMut<State>,
    sim: Option<Res<SimulationWorker>>,
) {
    let Some(sim) = sim else { return };

    if let Some(snap) = sim.latest_snapshot.lock().unwrap().take() {
        *world = snap.grid;
        state.simulation_step = snap.step;
    }

    let camera_scale = camera.iter().next().map(|t| t.scale.x).unwrap_or(1.0);
    let mip_blend = overview_blend(
        camera_scale,
        render_config.mip_lod_fade_start,
        render_config.mip_lod_fade_end,
    );

    // Build the composed world texture as soon as the smooth transition begins.
    // Its sampler uses linear minification + linear mip filtering, so once this
    // contribution becomes visible there is no nearest-neighbor shimmer.
    if mip_blend > 0.0 && overview.needs_overview_rebuild(&state) {
        overview.rebuild_overview(&world, &settings, &render_config, &state, &mut images);
    }

    // Once the transition has completed, detailed tile buffers no longer need
    // to be refreshed. They are updated exactly once when zooming back into the
    // blend/detail range.
    if mip_blend >= 1.0 {
        return;
    }
    if overview.last_precise_step == Some(state.simulation_step) {
        return;
    }

    // Each map is selected by an explicit marker component. ECS iteration order
    // can no longer swap layers at startup.
    let mut organics_map = get_map(organics_handle.iter().next().unwrap(), &mut *map_materials);
    let mut life_map = get_map(life_handle.iter().next().unwrap(), &mut *map_materials);
    let mut pollution_map = get_map(pollution_handle.iter().next().unwrap(), &mut *map_materials);
    let mut soil_energy_map = get_map(soil_energy_handle.iter().next().unwrap(), &mut *map_materials);
    let mut energy_directions_map = get_map(
        energy_directions_handle.iter().next().unwrap(),
        &mut *map_materials,
    );

    for x in 0..settings.w {
        for y in 0..settings.h {
            let index = y as usize * settings.w as usize + x as usize;
            let cell = &world.cells()[index];

            let organics_texture = cell.soil.organics as u32;
            let life_texture = cell.life.texture_id(&world, index);
            let pollution_texture = cell.air.pollution as u32;
            let soil_energy_texture = ((cell.soil.energy * 255.0
                / render_config.soil_energy_render_max)
                as u32)
                .min(255);
            let energy_directions_texture = cell.life.energy_directions_texture_id();

            if organics_map.at(x, y) != organics_texture {
                organics_map.set(x, y, organics_texture);
            }
            if life_map.at(x, y) != life_texture {
                life_map.set(x, y, life_texture);
            }
            if pollution_map.at(x, y) != pollution_texture {
                pollution_map.set(x, y, pollution_texture);
            }
            if soil_energy_map.at(x, y) != soil_energy_texture {
                soil_energy_map.set(x, y, soil_energy_texture);
            }
            if energy_directions_map.at(x, y) != energy_directions_texture {
                energy_directions_map.set(x, y, energy_directions_texture);
            }
        }
    }

    overview.last_precise_step = Some(state.simulation_step);
}

fn sync_render_lod_visibility(
    camera: Query<&Transform, With<Camera>>,
    state: Res<State>,
    render_config: Res<RenderConfig>,
    mut layers: ParamSet<(
        Query<(&mut Visibility, &mut MapAttributes), With<OrganicsLayer>>,
        Query<(&mut Visibility, &mut MapAttributes), With<LifeLayer>>,
        Query<(&mut Visibility, &mut MapAttributes), With<PollutionLayer>>,
        Query<(&mut Visibility, &mut MapAttributes), With<SoilEnergyLayer>>,
        Query<(&mut Visibility, &mut MapAttributes), With<EnergyDirectionsLayer>>,
        Query<(&mut Visibility, &mut Sprite), With<FarOverviewLayer>>,
    )>,
) {
    let camera_scale = camera.iter().next().map(|t| t.scale.x).unwrap_or(1.0);
    let mip_alpha = overview_blend(
        camera_scale,
        render_config.mip_lod_fade_start,
        render_config.mip_lod_fade_end,
    );
    let detail_alpha = 1.0 - mip_alpha;

    // Crossfade both renderers instead of flipping Visibility at one exact zoom.
    // FastTileMap's shader multiplies its sampled tile color by MapAttributes::mix_color,
    // so alpha here fades the complete detailed layer without changing tile data.
    {
        let mut query = layers.p0();
        set_detail_layer(&mut query, state.organic_visible, detail_alpha);
    }
    {
        let mut query = layers.p1();
        set_detail_layer(&mut query, state.life_visible, detail_alpha);
    }
    {
        let mut query = layers.p2();
        set_detail_layer(&mut query, state.pollution_visible, detail_alpha);
    }
    {
        let mut query = layers.p3();
        set_detail_layer(&mut query, state.soil_energy_visible, detail_alpha);
    }
    {
        let mut query = layers.p4();
        set_detail_layer(&mut query, state.energy_directions_visible, detail_alpha);
    }
    {
        let mut query = layers.p5();
        for (mut visibility, mut sprite) in query.iter_mut() {
            let visible = mip_alpha > 0.001;
            let desired = if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            if *visibility != desired {
                *visibility = desired;
            }

            let desired_color = Color::srgba(1.0, 1.0, 1.0, mip_alpha);
            if sprite.color != desired_color {
                sprite.color = desired_color;
            }
        }
    }
}

fn set_detail_layer<M: Component>(
    query: &mut Query<(&mut Visibility, &mut MapAttributes), With<M>>,
    enabled: bool,
    alpha: f32,
) {
    let visible = enabled && alpha > 0.001;
    let desired_visibility = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let color = Vec4::new(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0));

    for (mut visibility, mut attributes) in query.iter_mut() {
        if *visibility != desired_visibility {
            *visibility = desired_visibility;
        }

        // FastTileMap's managed mesh is a single triangle. Supplying one color
        // per vertex keeps alpha spatially uniform and avoids a transition edge.
        let needs_color = attributes.mix_color.len() != 3
            || attributes.mix_color.iter().any(|existing| *existing != color);
        if needs_color {
            attributes.mix_color.clear();
            attributes.mix_color.resize(3, color);
        }
    }
}
