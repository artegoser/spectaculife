use crate::cells::{
    life_cell::{AliveCell, EnergyDirections, LifeCell, LifeType::*},
    soil_cell::MAX_ENERGY_LIFE,
    WorldCell,
};
use crate::grid::{Area, Grid};
use crate::types::{Settings, State};
use crate::update::update_world;
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
            .add_systems(FixedUpdate, next_step.run_if(not_paused))
            .add_systems(FixedUpdate, initialize.run_if(not_initialized))
            // Resources
            .insert_resource(Time::<Fixed>::from_seconds(0.1))
            .insert_resource(Grid::<WorldCell>::default())
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

fn initialize(
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    mut state: ResMut<State>,
) {
    for x in 0..settings.w {
        for y in 0..settings.h {
            let cell = world.get_mut(x as i64, y as i64);
            *cell = WorldCell::default();

            if x == 127 && y == 127 {
                // if x % 2 == 0 && y % 2 == 0 {
                let life_cell =
                    AliveCell::new(Stem(rand::random()), 8., EnergyDirections::default(), None);
                cell.life = LifeCell::Alive(life_cell);
            }
        }
    }

    state.initialized = true;
}

fn not_paused(state: Res<State>) -> bool {
    !state.paused
}

fn not_initialized(state: Res<State>) -> bool {
    !state.initialized
}

pub fn next_step(
    mut map_materials: ResMut<Assets<Map>>,
    maps: Query<&Handle<Map>>,
    mut world: ResMut<Grid<WorldCell>>,
    settings: Res<Settings>,
    state: ResMut<State>,
) {
    let mut organics_map = get_map(&maps, &mut *map_materials, 0);
    let mut life_map = get_map(&maps, &mut *map_materials, 1);
    let mut pollution_map = get_map(&maps, &mut *map_materials, 2);
    let mut soil_energy_map = get_map(&maps, &mut *map_materials, 3);
    let mut energy_directions_map = get_map(&maps, &mut *map_materials, 4);

    for x in &state.cell_order_x {
        for y in &state.cell_order_y {
            let mut area = Area::new(&mut *world, *x, *y);
            update_world(&mut area);
        }
    }

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
