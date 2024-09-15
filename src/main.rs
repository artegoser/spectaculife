use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use components::life_cell::{LifeCell, LifeCellType};
use rand::Rng;

mod components;
mod helpers;
mod types;
mod utils;

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let texture_handle: Handle<Image> = asset_server.load("tiles.png");

    let map_size = TilemapSize { x: 128, y: 128 };

    let mut soil_storage = TileStorage::empty(map_size);
    let soil_entity = commands.spawn_empty().id();

    let mut rng = rand::thread_rng();
    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(soil_entity),
                    visible: TileVisible(true),
                    texture_index: TileTextureIndex(16),
                    // color: TileColor(Color::srgba(0., 1., 0., 0.5)),
                    ..Default::default()
                })
                .id();
            soil_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_pos = TilePos { x: 64, y: 64 };
    commands
        .entity(soil_storage.get(&tile_pos).unwrap())
        .insert(LifeCell::new(LifeCellType::Cancer, 5., None));

    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::Square;

    commands.entity(soil_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: soil_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });
}

fn update(
    mut commands: Commands,
    mut tile_storage_query: Query<(&TileStorage, &TilemapSize)>,
    tile_query: Query<(Entity, &TilePos, &TileTextureIndex, &mut LifeCell)>,
) {
    let (tile_storage, map_size) = tile_storage_query.single_mut();

    for (entity, position, texture_id, life_cell) in tile_query.iter() {
        // let neighbor_count = Neighbors::get_square_neighboring_positions(position, map_size, true)
        //     .entities(tile_storage)
        //     .iter()
        //     .filter(|neighbor| {
        //         let (_, _, index) = tile_query.get(**neighbor).unwrap();
        //         *index == TileTextureIndex(1)
        //     })
        //     .count();

        let new_texture_id = life_cell.texture_id();

        if new_texture_id != *texture_id {
            commands.entity(entity).insert(new_texture_id);
        };
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: String::from("SpectacuLife"),
                        ..Default::default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(TilemapPlugin)
        .add_systems(Startup, startup)
        .add_systems(Update, helpers::camera::movement)
        // .add_systems(Update, update)
        .add_systems(FixedUpdate, update)
        .insert_resource(Time::<Fixed>::from_seconds(0.05))
        .run();
}
