use crate::cells::{
    life_cell::{AliveCell, EnergyDirections, LifeCell, LifeType::*},
    WorldCell,
};
use crate::grid::{Area, Grid};
use crate::types::{Settings, State};
use crate::update::update_area;
use crate::utils::get_map;
use bevy::math::{uvec2, vec2, vec3};
use bevy::prelude::*;
use bevy_fast_tilemap::{FastTileMapPlugin, Map, MapBundleManaged};

#[derive(Default)]
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            // Plugins
            .add_plugins(FastTileMapPlugin::default())
            // Systems
            .add_systems(Startup, startup)
            .add_systems(FixedUpdate, update.run_if(not_paused))
            .add_systems(FixedUpdate, initialize.run_if(restart))
            // Resources
            .insert_resource(Time::<Fixed>::from_seconds(0.08))
            .insert_resource(Grid::<WorldCell>::new(0, 0))
            .insert_resource(Settings { w: 256, h: 256 })
            .insert_resource(State::default());
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
    *state = State::from_settings(&settings);

    let cell_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("life.png"),
        vec2(16., 16.),
    )
    .build();

    let soil_map = Map::builder(
        uvec2(settings.w, settings.h),
        asset_server.load("soil.png"),
        vec2(16., 16.),
    )
    .build();

    commands.spawn(MapBundleManaged {
        material: materials.add(soil_map),
        ..default()
    });

    commands.spawn(MapBundleManaged {
        material: materials.add(cell_map),
        transform: Transform::default().with_translation(vec3(0., 0., 1.)),
        ..default()
    });
}

fn initialize(
    mut map_materials: ResMut<Assets<Map>>,
    maps: Query<&Handle<Map>>,
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    mut state: ResMut<State>,
) {
    state.restart = false;

    let mut soil_map = get_map(&maps, &mut *map_materials, 0);
    let mut life_map = get_map(&maps, &mut *map_materials, 1);

    for x in 0..settings.w {
        for y in 0..settings.h {
            let cell = world.get_mut(x as i64, y as i64);
            *cell = WorldCell::default();

            soil_map.set(x, y, 0);
            life_map.set(x, y, 0);

            if x % 2 == 0 && y % 2 == 0 {
                let life_cell = AliveCell::new(
                    Stem(rand::random()),
                    500.,
                    None,
                    EnergyDirections::default(),
                );
                cell.life = LifeCell::Alive(life_cell);
            }
        }
    }
}

fn not_paused(state: Res<State>) -> bool {
    !state.paused
}

fn restart(state: Res<State>) -> bool {
    state.restart
}

fn update(
    mut map_materials: ResMut<Assets<Map>>,
    maps: Query<&Handle<Map>>,
    mut life: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    state: ResMut<State>,
) {
    let mut soil_map = get_map(&maps, &mut *map_materials, 0);
    let mut life_map = get_map(&maps, &mut *map_materials, 1);

    for x in &state.cell_order_x {
        for y in &state.cell_order_y {
            update_area(Area::new(&mut *life, *x, *y));
        }
    }

    for x in 0..settings.w {
        for y in 0..settings.h {
            let area = Area::new(&mut *life, x, y);

            let life_texture = area.center.life.texture_id(&area);
            let soil_texture = area.center.soil.texture_id();

            if life_map.at(x, y) != life_texture {
                life_map.set(x, y, life_texture);
            }

            if soil_map.at(x, y) != soil_texture {
                soil_map.set(x, y, soil_texture);
            }
        }
    }
}
