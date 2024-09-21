mod cells;
mod grid;
mod plugins;
mod types;
mod update;
mod utils;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use plugins::{control, world::WorldPlugin};

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
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
            control::ControlPlugin::default(),
            WorldPlugin::default(),
        ))
        .run();
}
