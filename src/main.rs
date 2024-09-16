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
use types::Settings;
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

    let map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("tiles.png"),
        vec2(16., 16.),
    )
    .build_and_initialize(|m| {
        for x in 0..settings.w {
            for y in 0..settings.h {
                if x == 64 && y == 64 {
                    let cell = world.get_mut(x as i64, y as i64);
                    let life_cell =
                        AliveCell::new(Cancer, 50000., None, EnergyDirections::default());
                    m.set(x, y, life_cell.texture_id());
                    cell.life = LifeCell::Alive(life_cell);
                    continue;
                }

                m.set(x, y, 15);
            }
        }
    });

    commands.spawn(MapBundleManaged {
        material: materials.add(map),
        ..default()
    });
}

fn update(
    mut map_materials: ResMut<Assets<Map>>,
    map: Query<&Handle<Map>>,
    mut life: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
) {
    let mut map = {
        let Some(map) = map_materials.get_mut(map.single()) else {
            warn!("No map material");
            return;
        };

        map.indexer_mut()
    };

    for x in 0..settings.w {
        for y in 0..settings.h {
            let prev_area = Area::new(&mut life, x, y);
            let new_area = update_area(prev_area.clone());

            macro_rules! check_update {
                ($id: ident) => {
                    if prev_area.$id != new_area.$id {
                        let coord = new_area.$id(&settings);

                        map.set(coord.x, coord.y, new_area.$id.life.texture_id());
                        life.uset(coord.x, coord.y, new_area.$id);
                    }
                };
            }

            check_update!(up);
            check_update!(down);
            check_update!(left);
            check_update!(right);
            check_update!(center);
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
