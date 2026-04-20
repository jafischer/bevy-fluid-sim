use std::time::{Duration, Instant};

use bevy::app::AppExit;
use bevy::camera::Camera;
use bevy::input::ButtonInput;
use bevy::math::Vec2;
use bevy::prelude::{
    Entity, GlobalTransform, KeyCode, MessageReader, MessageWriter, MouseButton, Query, Res, Single, Transform, Window,
    With,
};
use bevy::window::{PrimaryWindow, WindowResized};

use crate::components::*;
use crate::messages::MessageText;
use crate::sim_struct::Simulation;

/// Handles mouse clicks to attract/repel particles.
pub fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    mut sim: Single<&mut Simulation>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras_query: Query<(&Camera, &GlobalTransform)>,
    mut messages: Single<&mut Notifications>,
) {
    if let Ok(window) = windows.single() {
        sim.interaction_input_point = None;

        let left_click = buttons.pressed(MouseButton::Left);
        let right_click = buttons.pressed(MouseButton::Right);
        if (left_click || right_click)
            && let Some(cursor_position) = window.cursor_position()
            && let Some((camera, camera_transform)) = cameras_query.iter().next()
            && let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor_position)
        {
            // Clear the welcome message, if it's still being displayed.
            if let Some(msg) = messages.messages.first()
                && msg.duration == Duration::MAX
            {
                messages.messages.clear();
            }

            sim.interaction_input_strength = sim.interaction_input_strength.abs() * if left_click { 1.0 } else { -1.0 };
            sim.interaction_input_point = Some(point);
        }
    }
}

/// Process keyboard shortcuts.
#[allow(clippy::too_many_arguments)] // No real choice about the number of arguments here. ECS gonna ECS.
pub fn handle_keypress(
    kb: Res<ButtonInput<KeyCode>>,
    mut app_exit: MessageWriter<AppExit>,
    mut sim: Single<&mut Simulation>,
    mut particle_query: Query<(&mut Transform, &mut Particle)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras_query: Query<(&Camera, &GlobalTransform)>,
    mut kb_cmds: Single<&mut KeyboardCommands>,
    mut messages: Single<&mut Notifications>,
) {
    if let Ok(window) = windows.single() {
        let now = Instant::now();
        let cursor_pos = if let Some(cursor_position) = window.cursor_position() {
            let Some((camera, camera_transform)) = cameras_query.iter().next() else {
                return;
            };

            // Calculate a world position based on the cursor's position.
            camera
                .viewport_to_world_2d(camera_transform, cursor_position)
                .unwrap_or(Vec2::splat(f32::MAX))
        } else {
            Vec2::splat(f32::MAX)
        };

        for key in kb.get_pressed() {
            // Clear the welcome message, if it's still being displayed.
            if let Some(msg) = messages.messages.first()
                && msg.duration == Duration::MAX
            {
                messages.messages.clear();
            }

            match key {
                // Esc / Q: quit the app
                KeyCode::Escape | KeyCode::KeyQ => {
                    app_exit.write(AppExit::Success);
                }

                // ?: display help
                KeyCode::Slash if kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight) => {
                    let mut kb_help: String = "Keyboard commands:".into();
                    // Are we already displaying it?
                    for message in &messages.messages {
                        if message.text.starts_with(&kb_help) {
                            return;
                        }
                    }

                    for (_key, cmd) in kb_cmds.commands.iter() {
                        kb_help.push('\n');
                        kb_help.push_str(&format!("{:5} - {}", cmd.key_text, cmd.description));
                    }
                    kb_help.push_str("\nEsc   - Quit");

                    messages.messages.push(MessageText {
                        text: kb_help,
                        start_time: Instant::now(),
                        duration: Duration::from_secs(5),
                    });
                }

                // Other key: check the command map.
                key => {
                    if let Some(command) = kb_cmds.commands.get_mut(key)
                        && now.duration_since(command.last_action_time) >= command.interval
                    {
                        command.last_action_time = now;
                        (command.action)(
                            &mut sim,
                            kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight),
                            &cursor_pos,
                            &mut particle_query,
                            &mut messages,
                        );
                    }
                }
            }
        }
    }
}

pub fn on_resize(
    mut resize_reader: MessageReader<WindowResized>,
    mut sim: Single<&mut Simulation>,
    windows: Query<Entity, With<PrimaryWindow>>,
) {
    if let Ok(primary) = windows.single() {
        for e in resize_reader.read() {
            // Only process resize for the primary window.
            if e.window == primary {
                sim.on_resize(e.width, e.height);
            }
        }
    }
}
