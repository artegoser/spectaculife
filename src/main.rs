use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::{uvec2, vec2},
    prelude::*,
};
use bevy_fast_tilemap::{FastTileMapPlugin, Map, MapBundleManaged};
use cells::{
    life_cell::{AliveCell, EnergyDirections, LifeCell, LifeType::*},
    WorldCell,
};
use grid::{Area, Grid};
use rand::seq::{IteratorRandom, SliceRandom};
use types::{CellDir::*, Settings};
use update::update_area;

mod cells;
mod grid;
mod helpers;
mod types;
mod update;
mod utils;

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<Map>>,
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
) {
    commands.spawn(Camera2dBundle::default());

    *world = Grid::<WorldCell>::new(settings.w, settings.h);

    let cell_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("cells.png"),
        vec2(16., 16.),
    )
    .build_and_initialize(|m| {
        for x in 0..settings.w {
            for y in 0..settings.h {
                if x == 64 && y == 64 {
                    let cell = world.get_mut(x as i64, y as i64);
                    let life_cell =
                        AliveCell::new(Cancer, 50000., None, EnergyDirections::default());
                    cell.life = LifeCell::Alive(life_cell);
                }

                m.set(x, y, 0);
            }
        }
    });

    let soil_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("soil.png"),
        vec2(16., 16.),
    )
    .build_and_initialize(|m| {
        for x in 0..settings.w {
            for y in 0..settings.h {
                m.set(x, y, 0);
            }
        }
    });

    commands.spawn(MapBundleManaged {
        material: materials.add(soil_map),
        ..default()
    });

    commands.spawn(MapBundleManaged {
        material: materials.add(cell_map),
        ..default()
    });
}

fn update(
    mut map_materials: ResMut<Assets<Map>>,
    map: Query<&Handle<Map>>,
    mut life: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
) {
    let mut rng = rand::thread_rng();

    let mut map = {
        let Some(map) = map_materials.get_mut(map.iter().nth(1).unwrap()) else {
            warn!("No map material");
            return;
        };

        map.indexer_mut()
    };

    let mut x_rand: Vec<u32> = (0..settings.w).collect();
    x_rand.shuffle(&mut rng);

    let mut y_rand: Vec<u32> = (0..settings.h).collect();
    y_rand.shuffle(&mut rng);

    for x in &x_rand {
        for y in &y_rand {
            let prev_area = Area::new(&mut life, *x, *y);
            let new_area = update_area(prev_area.clone());

            macro_rules! check_update {
                ($dir: expr) => {
                    let dir = $dir;
                    let prev_cell = prev_area.cell_from_dir(&dir);
                    let new_cell = new_area.cell_from_dir(&dir);
                    if prev_cell != new_cell {
                        let coord = new_area.coord_from_dir(&dir, &settings);
                        map.set(
                            coord.x,
                            coord.y,
                            new_cell
                                .life
                                .texture_id(Area::new(&mut life, coord.x, coord.y)),
                        );
                        life.uset(coord.x, coord.y, new_cell);
                    }
                };
            }

            check_update!(Up);
            check_update!(Down);
            check_update!(Left);
            check_update!(Right);

            if prev_area.center != new_area.center {
                let coord = new_area.get_center_coord(&settings);

                map.set(coord.x, coord.y, new_area.center.life.texture_id(new_area));
                life.uset(coord.x, coord.y, new_area.center);
            }
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Spectaculife"),
                    ..default()
                }),
                ..default()
            }),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            helpers::mouse_camera::MouseControlsCameraPlugin::default(),
            FastTileMapPlugin::default(),
        ))
        .add_systems(Startup, startup)
        .add_systems(FixedUpdate, update)
        .insert_resource(Time::<Fixed>::from_seconds(0.1))
        .insert_resource(Grid::<WorldCell>::new(0, 0))
        .insert_resource(Settings { w: 128, h: 128 })
        .run();
}
