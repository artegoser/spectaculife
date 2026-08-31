use crate::cells::{
    life_cell::{
        genome::{Genome, GenomePool},
        AliveCell, EnergyDirections, LifeCell, LifeType::*,
    },
    WorldCell,
};
use crate::config::SimulationConfig;
use crate::grid::{Area, Grid};
use crate::types::{Settings, State};
use crate::update::{update_environment, update_world, EnvironmentBuffers};
use crate::utils::get_map;
use bevy::math::{uvec2, vec2, vec3};
use bevy::prelude::*;
use bevy_fast_tilemap::prelude::*;
use rand::seq::SliceRandom;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::Duration;

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
            .add_systems(Update, receive_and_render)
            .insert_resource(Grid::<WorldCell>::default())
            .insert_resource(Settings { w, h })
            .insert_resource(State::default());
    }
}

#[derive(Resource)]
pub struct SimulationWorker {
    receiver: Mutex<mpsc::Receiver<SimSnapshot>>,
    command_sender: Mutex<mpsc::Sender<SimCommand>>,
    paused: Arc<AtomicBool>,
    step_requested: Arc<AtomicBool>,
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
) -> (mpsc::Receiver<SimSnapshot>, mpsc::Sender<SimCommand>) {
    let (snap_tx, snap_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();

    thread::spawn(move || {
        let mut grid = grid;
        let mut genomes = genomes;
        let mut step: usize = 0;
        let mut shuffle_x: Vec<u32> = (0..settings.w).collect();
        let mut shuffle_y: Vec<u32> = (0..settings.h).collect();
        let mut rng = rand::thread_rng();
        let mut sim_state = State::default();
        sim_state.initialized = true;
        sim_state.next_organism_id = next_organism_id;
        let mut environment_buffers = EnvironmentBuffers::new(&grid);

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
                        environment_buffers = EnvironmentBuffers::new(&grid);
                        reset_happened = true;
                    }
                }
            }

            if reset_happened
                && snap_tx
                    .send(SimSnapshot {
                        grid: grid.clone(),
                        step,
                    })
                    .is_err()
            {
                break;
            }

            let do_step = step_requested.swap(false, Ordering::Relaxed);
            let is_paused = paused.load(Ordering::Relaxed);
            if is_paused && !do_step {
                thread::sleep(Duration::from_millis(1));
                continue;
            }

            update_environment(&mut grid, &mut environment_buffers, &config);

            shuffle_x.shuffle(&mut rng);
            shuffle_y.shuffle(&mut rng);

            for x in &shuffle_x {
                for y in &shuffle_y {
                    {
                        let cell = grid.get_mut(*x as i64, *y as i64);
                        if let LifeCell::Alive(ref mut life) = cell.life {
                            if life.incoming_energy != 0.0 {
                                life.energy += life.incoming_energy;
                                life.incoming_energy = 0.0;
                            }
                        }
                    }

                    let mut area = Area::new(&mut grid as *mut _, *x, *y);
                    update_world(&mut sim_state, &mut area, &mut genomes, &config);
                }
            }

            step += 1;
            sim_state.simulation_step = step;

            if snap_tx
                .send(SimSnapshot {
                    grid: grid.clone(),
                    step,
                })
                .is_err()
            {
                break;
            }
        }
    });

    (snap_rx, cmd_tx)
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

                cell.life = LifeCell::Alive(AliveCell::new(
                    Stem(handle),
                    organism_id,
                    config.world.initial_stem_energy,
                    EnergyDirections::default(),
                    None,
                    config.world.initial_stem_lifespan,
                ));
            }
        }
    }

    next_organism_id
}

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<Map>>,
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

    let (snap_rx, cmd_tx) = spawn_sim_thread(
        world.clone(),
        genomes,
        *settings,
        (*config).clone(),
        paused.clone(),
        step_requested.clone(),
        next_organism_id,
    );

    commands.insert_resource(SimulationWorker {
        receiver: Mutex::new(snap_rx),
        command_sender: Mutex::new(cmd_tx),
        paused,
        step_requested,
    });

    let cell_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("life.png"),
        vec2(16., 16.),
    )
    .build();
    let organics_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("organics.png"),
        vec2(1., 1.),
    )
    .build();
    let pollution_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("pollution.png"),
        vec2(1., 1.),
    )
    .build();
    let soil_energy_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("soil_energy.png"),
        vec2(1., 1.),
    )
    .build();
    let energy_directions_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("energy_directions.png"),
        vec2(16., 16.),
    )
    .build();

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
    organics_handle: Query<&Handle<Map>, With<OrganicsLayer>>,
    life_handle: Query<&Handle<Map>, With<LifeLayer>>,
    pollution_handle: Query<&Handle<Map>, With<PollutionLayer>>,
    soil_energy_handle: Query<&Handle<Map>, With<SoilEnergyLayer>>,
    energy_directions_handle: Query<&Handle<Map>, With<EnergyDirectionsLayer>>,
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    config: Res<SimulationConfig>,
    mut state: ResMut<State>,
    sim: Option<Res<SimulationWorker>>,
) {
    let Some(sim) = sim else { return };

    let mut latest: Option<SimSnapshot> = None;
    let receiver = sim.receiver.lock().unwrap();
    while let Ok(snap) = receiver.try_recv() {
        latest = Some(snap);
    }
    drop(receiver);

    if let Some(snap) = latest {
        *world = snap.grid;
        state.simulation_step = snap.step;
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
            let area = Area::new(&mut *world, x, y);

            let organics_texture = area.center.soil.organics as u32;
            let life_texture = area.center.life.texture_id(&area);
            let pollution_texture = area.center.air.pollution as u32;
            let soil_energy_texture = ((area.center.soil.energy * 255.0
                / config.environment.soil_energy_render_max)
                as u32)
                .min(255);
            let energy_directions_texture = area.center.life.energy_directions_texture_id();

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
}
