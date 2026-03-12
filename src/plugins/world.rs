use crate::cells::{
    life_cell::{
        genome::GenomePool,
        AliveCell, EnergyDirections, LifeCell, LifeType::*,
    },
    soil_cell::MAX_ENERGY_LIFE,
    WorldCell,
};
use crate::grid::{Area, Grid};
use crate::types::{Settings, State};
use crate::update::update_world;
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

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            // Plugins
            .add_plugins(FastTileMapPlugin::default())
            // Systems
            .add_systems(Startup, startup)
            .add_systems(Update, receive_and_render)
            // Resources
            .insert_resource(Grid::<WorldCell>::default())
            .insert_resource(Settings { w: 512, h: 512 })
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

    pub fn reinitialize(&self, grid: Grid<WorldCell>, genomes: GenomePool) {
        let _ = self.command_sender.lock().unwrap().send(SimCommand::SetGrid(grid, genomes));
    }
}

struct SimSnapshot {
    grid: Grid<WorldCell>,
    step: usize,
}

enum SimCommand {
    SetGrid(Grid<WorldCell>, GenomePool),
}

fn spawn_sim_thread(
    grid: Grid<WorldCell>,
    genomes: GenomePool,
    settings: Settings,
    paused: Arc<AtomicBool>,
    step_requested: Arc<AtomicBool>,
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

        loop {
            // Check for commands
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    SimCommand::SetGrid(new_grid, new_genomes) => {
                        grid = new_grid;
                        genomes = new_genomes;
                        step = 0;
                        sim_state.simulation_step = 0;
                    }
                }
            }

            // Check if we should simulate
            let do_step = step_requested.swap(false, Ordering::Relaxed);
            let is_paused = paused.load(Ordering::Relaxed);

            if is_paused && !do_step {
                thread::sleep(Duration::from_millis(1));
                continue;
            }

            // Merged loop: incoming_energy + update in shuffled order
            shuffle_x.shuffle(&mut rng);
            shuffle_y.shuffle(&mut rng);

            for x in &shuffle_x {
                for y in &shuffle_y {
                    // Process incoming energy
                    {
                        let cell = grid.get_mut(*x as i64, *y as i64);
                        if let LifeCell::Alive(ref mut life) = cell.life {
                            if life.incoming_energy != 0.0 {
                                life.energy += life.incoming_energy;
                                life.incoming_energy = 0.0;
                            }
                        }
                    }

                    // Update world
                    let mut area = Area::new(&mut grid as *mut _, *x, *y);
                    update_world(&mut sim_state, &mut area, &mut genomes);
                }
            }

            step += 1;
            sim_state.simulation_step = step;

            // Send snapshot (clone grid for main thread rendering)
            if snap_tx
                .send(SimSnapshot {
                    grid: grid.clone(),
                    step,
                })
                .is_err()
            {
                break; // main thread dropped receiver, exit
            }
        }
    });

    (snap_rx, cmd_tx)
}

fn populate_grid(
    grid: &mut Grid<WorldCell>,
    genomes: &mut GenomePool,
    settings: &Settings,
) {
    for x in 0..settings.w {
        for y in 0..settings.h {
            let cell = grid.get_mut(x as i64, y as i64);
            *cell = WorldCell::default();

            if x % 4 == 0 && y % 4 == 0 {
                let genome: crate::cells::life_cell::genome::Genome = rand::random();
                let handle = genomes.alloc(genome);
                let life_cell = AliveCell::new(
                    Stem(handle),
                    100.,
                    EnergyDirections::default(),
                    None,
                    2,
                );
                cell.life = LifeCell::Alive(life_cell);
            }
        }
    }
}

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<Map>>,
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    mut state: ResMut<State>,
) {
    commands.spawn(Camera2dBundle::default());

    *world = Grid::<WorldCell>::new(settings.w, settings.h);

    // Initialize grid
    let mut genomes = GenomePool::new();
    populate_grid(&mut world, &mut genomes, &settings);
    state.initialized = true;

    // Spawn simulation thread
    let paused = Arc::new(AtomicBool::new(false));
    let step_requested = Arc::new(AtomicBool::new(false));

    let (snap_rx, cmd_tx) = spawn_sim_thread(
        world.clone(),
        genomes,
        *settings,
        paused.clone(),
        step_requested.clone(),
    );

    commands.insert_resource(SimulationWorker {
        receiver: Mutex::new(snap_rx),
        command_sender: Mutex::new(cmd_tx),
        paused,
        step_requested,
    });

    // Create tilemaps
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

    commands.spawn(MapBundleManaged {
        material: materials.add(organics_map),
        transform: Transform::default().with_scale(vec3(16., 16., 1.)),
        ..default()
    });

    commands.spawn(MapBundleManaged {
        material: materials.add(cell_map),
        transform: Transform::default().with_translation(vec3(0., 0., 2.)),
        ..default()
    });

    commands.spawn(MapBundleManaged {
        material: materials.add(pollution_map),
        transform: Transform::default()
            .with_translation(vec3(0., 0., 4.))
            .with_scale(vec3(16., 16., 1.)),
        ..default()
    });

    commands.spawn(MapBundleManaged {
        material: materials.add(soil_energy_map),
        transform: Transform::default()
            .with_translation(vec3(0., 0., 1.))
            .with_scale(vec3(16., 16., 1.)),
        ..default()
    });

    commands.spawn(MapBundleManaged {
        material: materials.add(energy_directions_map),
        transform: Transform::default().with_translation(vec3(0., 0., 3.)),
        ..default()
    });
}

fn receive_and_render(
    mut map_materials: ResMut<Assets<Map>>,
    maps: Query<&Handle<Map>>,
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    mut state: ResMut<State>,
    sim: Option<Res<SimulationWorker>>,
) {
    let Some(sim) = sim else { return };

    // Drain channel to get latest snapshot
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

    // Render tilemaps from current world state
    let mut organics_map = get_map(&maps, &mut *map_materials, 0);
    let mut life_map = get_map(&maps, &mut *map_materials, 1);
    let mut pollution_map = get_map(&maps, &mut *map_materials, 2);
    let mut soil_energy_map = get_map(&maps, &mut *map_materials, 3);
    let mut energy_directions_map = get_map(&maps, &mut *map_materials, 4);

    for x in 0..settings.w {
        for y in 0..settings.h {
            let area = Area::new(&mut *world, x, y);

            let organics_texture = area.center.soil.organics as u32;
            let life_texture = area.center.life.texture_id(&area);
            let pollution_texture = area.center.air.pollution as u32;
            let soil_energy_texture =
                ((area.center.soil.energy * 255. / MAX_ENERGY_LIFE) as u32).min(255);

            let energy_directions_texturue = area.center.life.energy_directions_texture_id();

            if organics_map.at(x, y) != organics_texture && state.organic_visible {
                organics_map.set(x, y, organics_texture);
            }

            if life_map.at(x, y) != life_texture && state.life_visible {
                life_map.set(x, y, life_texture);
            }

            if pollution_map.at(x, y) != pollution_texture && state.pollution_visible {
                pollution_map.set(x, y, pollution_texture);
            }

            if soil_energy_map.at(x, y) != soil_energy_texture {
                soil_energy_map.set(x, y, soil_energy_texture);
            }

            if energy_directions_map.at(x, y) != energy_directions_texturue {
                energy_directions_map.set(x, y, energy_directions_texturue);
            }
        }
    }
}
