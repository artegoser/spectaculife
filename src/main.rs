use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::Neighbors;
use bevy_ecs_tilemap::prelude::*;
use rand::Rng;

mod helpers;

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let texture_handle: Handle<Image> = asset_server.load("tiles.png");

    let map_size = TilemapSize { x: 128, y: 128 };
    let mut tile_storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    let mut rng = rand::thread_rng();
    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    visible: TileVisible(true),
                    texture_index: TileTextureIndex(rng.gen::<bool>() as u32),
                    ..Default::default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::Square;

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });
}

fn update(
    mut commands: Commands,
    mut tile_storage_query: Query<(&TileStorage, &TilemapSize)>,
    tile_query: Query<(Entity, &TilePos, &TileTextureIndex)>,
) {
    let (tile_storage, map_size) = tile_storage_query.single_mut();
    for (entity, position, index) in tile_query.iter() {
        let neighbor_count = Neighbors::get_square_neighboring_positions(position, map_size, true)
            .entities(tile_storage)
            .iter()
            .filter(|neighbor| {
                let (_, _, index) = tile_query.get(**neighbor).unwrap();
                *index == TileTextureIndex(1)
            })
            .count();

        let was_alive = *index == TileTextureIndex(1);

        let is_alive = match (was_alive, neighbor_count) {
            (true, x) if x < 2 => false,
            (true, 2) | (true, 3) => true,
            (true, x) if x > 3 => false,
            (false, 3) => true,
            (otherwise, _) => otherwise,
        };

        if is_alive && !was_alive {
            commands.entity(entity).insert(TileTextureIndex(1));
        } else if !is_alive && was_alive {
            commands.entity(entity).insert(TileTextureIndex(0));
        }
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
