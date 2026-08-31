use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    math::{uvec2, vec3},
    prelude::*,
};
use bevy_fast_tilemap::prelude::*;

use crate::{
    config::{
        DirectionActionKind, RenderConfig, SimulationConfig, DEFAULT_CONFIG_PATH,
        DEFAULT_RENDER_CONFIG_PATH,
    },
    types::State,
};

use super::{
    overview::overview_blend,
    world::{LifeLayer, SimulationWorker},
};

#[derive(Default)]
pub struct ControlPlugin;

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct HudText;

#[derive(Debug, Clone, Copy, Event)]
enum ControlAction {
    TogglePause,
    Step,
    Reset,
    ToggleOrganics,
    ToggleLife,
    TogglePollution,
    ToggleSoilEnergy,
    ToggleEnergyDirections,
    ToggleHud,
}

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ControlAction>()
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (
                    keyboard_input,
                    hud_button_input,
                    apply_control_actions,
                    mouse_controls_camera,
                    update_cursor_position,
                    update_hud,
                )
                    .chain(),
            );
    }
}

fn keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut actions: EventWriter<ControlAction>) {
    let shortcuts = [
        (KeyCode::Space, ControlAction::TogglePause),
        (KeyCode::KeyN, ControlAction::Step),
        (KeyCode::KeyI, ControlAction::Reset),
        (KeyCode::KeyO, ControlAction::ToggleOrganics),
        (KeyCode::KeyL, ControlAction::ToggleLife),
        (KeyCode::KeyP, ControlAction::TogglePollution),
        (KeyCode::KeyS, ControlAction::ToggleSoilEnergy),
        (KeyCode::KeyD, ControlAction::ToggleEnergyDirections),
        (KeyCode::KeyH, ControlAction::ToggleHud),
    ];

    for (key, action) in shortcuts {
        if keys.just_pressed(key) {
            actions.send(action);
        }
    }
}

fn hud_button_input(
    interactions: Query<(&Interaction, &ControlAction), (Changed<Interaction>, With<Button>)>,
    mut actions: EventWriter<ControlAction>,
) {
    for (interaction, action) in &interactions {
        if *interaction == Interaction::Pressed {
            actions.send(*action);
        }
    }
}

fn apply_control_actions(
    mut actions: EventReader<ControlAction>,
    mut state: ResMut<State>,
    mut hud: Query<&mut Visibility, With<HudRoot>>,
    sim: Option<Res<SimulationWorker>>,
) {
    for action in actions.read().copied() {
        match action {
            ControlAction::TogglePause => {
                state.paused = !state.paused;
                if let Some(sim) = &sim {
                    sim.set_paused(state.paused);
                }
            }
            ControlAction::Step => {
                if let Some(sim) = &sim {
                    sim.request_step();
                }
            }
            ControlAction::Reset => {
                if let Some(sim) = &sim {
                    sim.reinitialize();
                    state.simulation_step = 0;
                }
            }
            ControlAction::ToggleOrganics => {
                state.organic_visible = !state.organic_visible;
            }
            ControlAction::ToggleLife => {
                state.life_visible = !state.life_visible;
            }
            ControlAction::TogglePollution => {
                state.pollution_visible = !state.pollution_visible;
            }
            ControlAction::ToggleSoilEnergy => {
                state.soil_energy_visible = !state.soil_energy_visible;
            }
            ControlAction::ToggleEnergyDirections => {
                state.energy_directions_visible = !state.energy_directions_visible;
            }
            ControlAction::ToggleHud => {
                state.hud_visible = !state.hud_visible;
                set_visibility(&mut hud, state.hud_visible);
            }
        }
    }
}

fn set_visibility<M: Component>(query: &mut Query<&mut Visibility, With<M>>, visible: bool) {
    for mut visibility in query.iter_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    left: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    max_width: Val::Px(760.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.02, 0.025, 0.035, 0.88)),
                ..default()
            },
            HudRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "Spectaculife",
                    TextStyle {
                        font_size: 14.0,
                        color: Color::srgb(0.92, 0.94, 0.98),
                        ..default()
                    },
                ),
                HudText,
            ));

            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        margin: UiRect {
                            top: Val::Px(8.0),
                            ..default()
                        },
                        ..default()
                    },
                    ..default()
                })
                .with_children(|buttons| {
                    macro_rules! hud_button {
                        ($label:expr, $action:expr) => {{
                            buttons
                                .spawn((
                                    ButtonBundle {
                                        style: Style {
                                            padding: UiRect {
                                                left: Val::Px(7.0),
                                                right: Val::Px(7.0),
                                                top: Val::Px(4.0),
                                                bottom: Val::Px(4.0),
                                            },
                                            margin: UiRect {
                                                right: Val::Px(4.0),
                                                ..default()
                                            },
                                            ..default()
                                        },
                                        background_color: BackgroundColor(Color::srgba(
                                            0.12, 0.14, 0.19, 0.95,
                                        )),
                                        ..default()
                                    },
                                    $action,
                                ))
                                .with_children(|button| {
                                    button.spawn(TextBundle::from_section(
                                        $label,
                                        TextStyle {
                                            font_size: 11.0,
                                            color: Color::srgb(0.92, 0.94, 0.98),
                                            ..default()
                                        },
                                    ));
                                });
                        }};
                    }

                    hud_button!("Pause [Space]", ControlAction::TogglePause);
                    hud_button!("Step [N]", ControlAction::Step);
                    hud_button!("Reset [I]", ControlAction::Reset);
                    hud_button!("Organics [O]", ControlAction::ToggleOrganics);
                    hud_button!("Life [L]", ControlAction::ToggleLife);
                    hud_button!("Pollution [P]", ControlAction::TogglePollution);
                    hud_button!("Soil [S]", ControlAction::ToggleSoilEnergy);
                    hud_button!("Paths [D]", ControlAction::ToggleEnergyDirections);
                });
        });
}

fn update_hud(
    state: Res<State>,
    config: Res<SimulationConfig>,
    render_config: Res<RenderConfig>,
    sim: Option<Res<SimulationWorker>>,
    camera: Query<&Transform, With<Camera>>,
    mut text_query: Query<&mut Text, With<HudText>>,
    mut hud_frame: Local<u8>,
) {
    *hud_frame = hud_frame.wrapping_add(1);
    if *hud_frame & 7 != 0 {
        return;
    }

    let status = if state.paused { "PAUSED" } else { "RUNNING" };
    let on_off = |value: bool| if value { "on" } else { "off" };
    let (tick_ms, ticks_per_second) = sim
        .as_ref()
        .map(|sim| (sim.average_tick_ms(), sim.ticks_per_second()))
        .unwrap_or((0.0, 0.0));
    let camera_scale = camera.iter().next().map(|t| t.scale.x).unwrap_or(1.0);
    let mip_blend = overview_blend(
        camera_scale,
        render_config.mip_lod_fade_start,
        render_config.mip_lod_fade_end,
    );
    let render_mode = if mip_blend <= 0.001 {
        "tile detail"
    } else if mip_blend >= 0.999 {
        "trilinear mip"
    } else {
        "tile -> mip blend"
    };
    let transfer_cap = config
        .life
        .transfer
        .max_energy_per_tick
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "none".to_string());
    let total_direction_weight: u64 = config
        .genetics
        .direction_actions
        .iter()
        .map(|entry| entry.weight as u64)
        .sum();
    let multiply_weight: u64 = config
        .genetics
        .direction_actions
        .iter()
        .filter(|entry| matches!(entry.kind, DirectionActionKind::MultiplySelf))
        .map(|entry| entry.weight as u64)
        .sum();
    let somatic_branching = if total_direction_weight == 0 {
        0.0
    } else {
        4.0 * multiply_weight as f32 / total_direction_weight as f32
    };

    let value = format!(
        "Spectaculife  |  {status}  |  step {}  |  {:.3} ms/tick  |  {:.1} ticks/s  |  cursor {},{}\n\
Render: {}  |  camera scale {:.2}  |  mip blend {:>3.0}%  |  fade {:.1}..{:.1}\n\
Layers: [O] organics {}   [L] life {}   [P] pollution {}   [S] soil energy {}   [D] energy paths {}\n\
World: {}x{}   spawn spacing {}   initial soil {:.1}..{:.1}   soil diffusion {:.2}   air diffusion {:.2}\n\
Life: leaf +{:.2}/tick   transfer cap {}   collision self/foreign {}/{}   seed {:.2}->{:.1} charge<={:.2}/tick\n\
Genetics: somatic branching E={:.2}   seed burst {}/gene   somatic {:.3}%@rate100 x{}   lifespan {}..{}   initial mutation {}..{}%   mutation bounds {}..{}%\n\
Hotkeys: [Space] pause/resume   [N] single step   [I] reset   [O/L/P/S/D] layers   [H] HUD\n\
Mouse: LMB/RMB drag   wheel zoom\n\
Configs: simulation={}   render={}",
        state.simulation_step,
        tick_ms,
        ticks_per_second,
        state.cursor_position.x,
        state.cursor_position.y,
        render_mode,
        camera_scale,
        mip_blend * 100.0,
        render_config.mip_lod_fade_start,
        render_config.mip_lod_fade_end,
        on_off(state.organic_visible),
        on_off(state.life_visible),
        on_off(state.pollution_visible),
        on_off(state.soil_energy_visible),
        on_off(state.energy_directions_visible),
        config.world.width,
        config.world.height,
        config.world.organism_spacing,
        config.world.initial_soil_energy.min,
        config.world.initial_soil_energy.max,
        config.environment.soil_diffusion,
        config.environment.air_diffusion,
        config.life.generators.leaf.energy_per_tick,
        transfer_cap,
        config.life.collision.self_damage,
        config.life.collision.foreign_damage,
        config.life.reproduction.seed_initial_energy,
        config.life.reproduction.seed_maturation_energy,
        config.life.reproduction.seed_max_charge_per_tick,
        somatic_branching,
        config.genetics.seed_mutation.edits_per_affected_gene,
        config.genetics.somatic_mutation.chance_per_million as f32 / 10_000.0,
        config.genetics.somatic_mutation.edits,
        config.genetics.lifespan.min,
        config.genetics.lifespan.max,
        config.genetics.initial_mutation_rate.min,
        config.genetics.initial_mutation_rate.max,
        config.genetics.mutation_rate_min,
        config.genetics.mutation_rate_max,
        DEFAULT_CONFIG_PATH,
        DEFAULT_RENDER_CONFIG_PATH,
    );

    for mut text in &mut text_query {
        text.sections[0].value.clone_from(&value);
    }
}

/// Use LMB/RMB for panning and the scroll wheel for zooming.
fn mouse_controls_camera(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut camera_query: Query<(
        &GlobalTransform,
        &mut Transform,
        &Camera,
        &mut OrthographicProjection,
    )>,
) {
    for event in mouse_motion_events.read() {
        if mouse_button.pressed(MouseButton::Left) || mouse_button.pressed(MouseButton::Right) {
            for (_, mut transform, _, _) in camera_query.iter_mut() {
                transform.translation.x -= event.delta.x * transform.scale.x;
                transform.translation.y += event.delta.y * transform.scale.y;
            }
        }
    }

    let mut wheel_y = 0.;
    for event in mouse_wheel_events.read() {
        wheel_y += event.y;
    }

    if wheel_y != 0. {
        for (_, mut transform, _, mut _ortho) in camera_query.iter_mut() {
            let factor = f32::powf(2., -wheel_y / 2.);
            transform.scale *= vec3(factor, factor, 1.0);
            transform.scale = transform
                .scale
                .max(Vec3::splat(1. / 128.))
                .min(Vec3::splat(128.));
        }
    }
}

fn update_cursor_position(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut camera_query: Query<(&GlobalTransform, &Camera), With<OrthographicProjection>>,
    mut state: ResMut<State>,
    life_map: Query<&Handle<Map>, With<LifeLayer>>,
    materials: Res<Assets<Map>>,
) {
    let Some(map_handle) = life_map.iter().next() else {
        return;
    };
    let Some(map) = materials.get(map_handle) else {
        return;
    };

    for event in cursor_moved_events.read() {
        for (global, camera) in camera_query.iter_mut() {
            if let Some(world) = camera
                .viewport_to_world(global, event.position)
                .map(|ray| ray.origin.truncate())
            {
                let coord = map
                    .world_to_map(world)
                    .as_uvec2()
                    .clamp(uvec2(0, 0), map.map_size() - uvec2(1, 1));

                state.cursor_position.x = coord.x;
                state.cursor_position.y = coord.y;
            }
        }
    }
}
