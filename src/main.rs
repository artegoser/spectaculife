mod cells;
mod grid;
mod plugins;
mod types;
mod update;
mod utils;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::{uvec2, vec2, vec3},
    prelude::*,
};
use bevy_fast_tilemap::{FastTileMapPlugin, Map};
use cells::{
    life_cell::{AliveCell, EnergyDirections, LifeCell, LifeType::*},
    WorldCell,
};
use grid::{Area, Grid};
use plugins::{control, world::WorldPlugin};
use rand::seq::SliceRandom;
use types::{CellDir::*, Settings, State};
use update::update_area;
use utils::get_map;

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
            control::ControlPlugin::default(),
            WorldPlugin::default(),
        ))
        // .insert_resource(Msaa::default())
        .run();
}
