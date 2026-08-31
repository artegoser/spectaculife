mod cells;
mod config;
mod grid;
mod plugins;
mod types;
mod update;
mod utils;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::{
        settings::{Backends, RenderCreation, WgpuSettings},
        RenderPlugin,
    },
};
use config::SimulationConfig;
use plugins::{control, world::WorldPlugin};

fn main() {
    let simulation_config = SimulationConfig::load();

    App::new()
        .insert_resource(simulation_config)
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: String::from("Spectaculife"),

                        ..default()
                    }),

                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(Backends::VULKAN),
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
