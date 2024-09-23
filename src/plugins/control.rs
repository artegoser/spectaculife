use bevy::{
    input::{
        common_conditions::{input_just_pressed, input_pressed},
        mouse::{MouseMotion, MouseWheel},
    },
    math::{uvec2, vec3},
    prelude::*,
};
use bevy_fast_tilemap::Map;

use crate::types::State;

use super::world::next_step;

#[derive(Default)]
pub struct ControlPlugin;

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                keyboard_input,
                mouse_controls_camera,
                update_cursor_position,
                next_step.run_if(input_just_pressed(KeyCode::KeyN)),
            ),
        );
    }
}

fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<State>,
    mut maps: Query<(&Handle<Map>, &mut Visibility)>,
) {
    if keys.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
    }

    if keys.just_pressed(KeyCode::KeyI) {
        state.initialized = false;
    }

    if keys.just_pressed(KeyCode::KeyO) {
        let (_, mut visibility) = maps.iter_mut().nth(0).unwrap();
        if state.organic_visible {
            *visibility = Visibility::Hidden;
            state.organic_visible = false;
        } else {
            *visibility = Visibility::Visible;
            state.organic_visible = true;
        }
    }

    if keys.just_pressed(KeyCode::KeyL) {
        let (_, mut visibility) = maps.iter_mut().nth(1).unwrap();
        if state.life_visible {
            *visibility = Visibility::Hidden;
            state.life_visible = false;
        } else {
            *visibility = Visibility::Visible;
            state.life_visible = true;
        }
    }

    if keys.just_pressed(KeyCode::KeyP) {
        let (_, mut visibility) = maps.iter_mut().nth(2).unwrap();
        if state.pollution_visible {
            *visibility = Visibility::Hidden;
            state.pollution_visible = false;
        } else {
            *visibility = Visibility::Visible;
            state.pollution_visible = true;
        }
    }
}

/// Use RMB for panning
/// Use scroll wheel for zooming
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
    maps: Query<&Handle<Map>>,

    materials: ResMut<Assets<Map>>,
) {
    for event in cursor_moved_events.read() {
        let map = materials.get(maps.iter().nth(1).unwrap()).unwrap();

        for (global, camera) in camera_query.iter_mut() {
            if let Some(world) = camera
                .viewport_to_world(global, event.position)
                .map(|ray| ray.origin.truncate())
            {
                let coord = map.world_to_map(world);

                let coord = coord
                    .as_uvec2()
                    .clamp(uvec2(0, 0), map.map_size() - uvec2(1, 1));

                state.cursor_position.x = coord.x;
                state.cursor_position.y = coord.y;
            }
        }
    }
}
