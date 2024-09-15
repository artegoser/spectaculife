use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::{uvec2, vec2},
    prelude::*,
};
use bevy_fast_tilemap::{FastTileMapPlugin, Map, MapBundleManaged};
use components::life_cell::{LifeCell, LifeCellType::*};
use grid::Grid;
use types::{CellDir::*, Settings};

mod components;
mod grid;
mod helpers;
mod types;
mod utils;

const HEIGHT: u32 = 128;
const WIDTH: u32 = 128;

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<Map>>,
    mut life: ResMut<Grid<Option<LifeCell>>>,
    settings: Res<Settings>,
) {
    commands.spawn(Camera2dBundle::default());

    let map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("tiles.png"),
        vec2(16., 16.),
    )
    .build_and_initialize(|m| {
        for x in 0..settings.w {
            for y in 0..settings.h {
                if x == 64 && y == 64 {
                    life.set(x as i64, y as i64, Some(LifeCell::new(Cancer, 5., None)));
                    m.set(x, y, 6);
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
    mut life: ResMut<Grid<Option<LifeCell>>>,
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
            if let Some(cell) = life.get_mut(x as i64, y as i64).clone() {
                match cell.cell {
                    Cancer => {
                        let up = life.get_mut(x as i64, y as i64 - 1);

                        *up = Some(LifeCell::new(Cancer, 5., Some(Down)))
                    }
                }

                map.set(x, y, cell.texture_id());
            };
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
        // Performance-wise you can step this much faster but it'd require an epillepsy warning.
        .insert_resource(Time::<Fixed>::from_seconds(0.2))
        .insert_resource(Grid::<Option<LifeCell>>::new(WIDTH, HEIGHT))
        .insert_resource(Settings {
            w: WIDTH,
            h: HEIGHT,
        })
        .run();
}
