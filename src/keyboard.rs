use std::collections::HashMap;
use std::time::{Duration, Instant};
use bevy::app::AppExit;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use crate::{MessageText, Messages};
use crate::particle::Particle;
use crate::sim::Simulation;

pub struct KeyboardCommand {
    pub description: String,
    pub last_action_time: Instant,
    pub interval: Duration,
    pub action: KeyboardAction,
}

type KeyboardAction = fn(
    &mut Simulation,
    // true == shift is pressed
    bool,
    // current mouse cursor position
    &Vec2,
    particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    messages: &mut Single<&mut Messages>,
);

#[derive(Component)]
pub struct KeyboardCommands {
    pub commands: HashMap<KeyCode, KeyboardCommand>,
}

impl KeyboardCommands {
    pub fn new() -> Self {
        let mut kb_cmds = KeyboardCommands {
            commands: HashMap::new(),
        };

        // Space: freeze / unfreeze particle motion.
        kb_cmds.add_command(KeyCode::Space, "Pause", 250, |sim, _, _, _, _| {
            if sim.frames_to_advance() == 0 {
                sim.set_frames_to_show(u32::MAX);
            } else {
                sim.set_frames_to_show(0);
            }
        });

        // 1: advance 1 frames.
        kb_cmds.add_command(KeyCode::Digit1, "Advance 1 frame", 500, |sim, _, _, _, _| sim.set_frames_to_show(1));
        // A: toggle velocity arrows
        // kb_cmds.add_command(KeyCode::KeyA, "Toggle velocity arrows", 250, |sim, _, _, _, _| sim.toggle_arrows());
        // C: toggle display of smoothing radius circle.
        kb_cmds.add_command(KeyCode::KeyC, "Draw smoothing radius around particle 0", 250, |sim, _, _, _, _| {
            sim.toggle_smoothing_radius()
        });
        // G: increase/decrease gravity
        kb_cmds.add_command(KeyCode::KeyG, "Decrease gravity (shift: increase)", 50, |sim, shift, _, _, msgs| {
            if shift {
                sim.adj_gravity(-0.5);
            } else {
                sim.adj_gravity(0.5);
            }
            msgs.messages.push(MessageText {
                text: Some(format!("Gravity: {:.1}", sim.gravity.y)),
                start_time: Instant::now(),
                duration: Duration::from_secs(1),
            });
        });
        // H: toggle heat map
        kb_cmds.add_command(KeyCode::KeyH, "Toggle heatmap", 250, |sim, _, _, _, msgs| {
            sim.toggle_heatmap();
            if sim.debug.use_heatmap {
                msgs.messages.push(MessageText {
                    text: Some("Density heatmap".into()),
                    start_time: Instant::now(),
                    duration: Duration::from_secs(1),
                });
            } else {
                msgs.messages.push(MessageText {
                    text: Some("Velocity heatmap".into()),
                    start_time: Instant::now(),
                    duration: Duration::from_secs(1),
                });
            }
        });
        // I: toggle inertia (see sim.calculate_pressure()).
        kb_cmds.add_command(KeyCode::KeyI, "Toggle inertia", 250, |sim, _, _, _, msgs| {
            sim.toggle_inertia();
            msgs.messages.push(MessageText {
                text: Some(format!("Inertia {}", if sim.debug.use_inertia { "on" } else { "off" })),
                start_time: Instant::now(),
                duration: Duration::from_secs(1),
            });
        });
        // L: log debug info in the next frame
        kb_cmds.add_command(KeyCode::KeyL, "Log debug info", 250, |sim, _, _, _, _| sim.log_next_frame());
        // R: reset the simulation
        kb_cmds.add_command(KeyCode::KeyR, "Reset particles", 250, |sim, _, _, _, _| sim.reset());
        // I: toggle inertia (see sim.calculate_pressure()).
        kb_cmds.add_command(KeyCode::KeyV, "Toggle inertia", 250, |sim, _, _, _, msgs| {
            sim.toggle_viscosity();
            msgs.messages.push(MessageText {
                text: Some(format!("Viscosity {}", if sim.debug.use_viscosity { "on" } else { "off" })),
                start_time: Instant::now(),
                duration: Duration::from_secs(1),
            });
        });

        // S: increase/decrease smoothing radius.
        kb_cmds.add_command(
            KeyCode::KeyS,
            "Decrease smoothing radius (shift: increase)",
            50,
            |sim, shift, _, _, msgs| {
                if shift {
                    sim.adj_smoothing_radius(0.01);
                } else {
                    sim.adj_smoothing_radius(-0.01);
                }
                msgs.messages.push(MessageText {
                    text: Some(format!("Smoothing radius: {:.2}", sim.smoothing_radius)),
                    start_time: Instant::now(),
                    duration: Duration::from_secs(1),
                });
            },
        );
        // W: "watch" the particle(s) under the cursor (color them yellow).
        // Shift-W: clear all watched particles.
        kb_cmds.add_command(
            KeyCode::KeyW,
            "Watch (highlight) particle under cursor",
            250,
            |sim, shift, cursor_pos, particle_query, _| {
                if shift {
                    particle_query
                        .par_iter_mut()
                        .for_each(|(_, mut particle)| particle.watched = false);
                } else {
                    particle_query.par_iter_mut().for_each(|(transform, mut particle)| {
                        if (transform.translation.xy() - cursor_pos).length() <= sim.particle_size {
                            println!(
                                "Watching particle {} @({},{}) density={}, velocity={:?}",
                                particle.id,
                                transform.translation.x,
                                transform.translation.y,
                                sim.densities[particle.id].0,
                                sim.velocities[particle.id]
                            );
                            particle.watched = true;
                        }
                    });
                }
            },
        );
        // X: toggle region grid
        kb_cmds.add_command(KeyCode::KeyX, "Display region grid", 500, |sim, _, _, _, _| sim.toggle_region_grid());

        kb_cmds
    }

    pub fn add_command(&mut self, key: KeyCode, description: &str, interval_millis: u64, action: KeyboardAction) {
        self.commands.insert(
            key,
            KeyboardCommand {
                description: description.into(),
                last_action_time: Instant::now(),
                interval: Duration::from_millis(interval_millis),
                action,
            },
        );
    }
}

pub fn handle_keypress(
    kb: Res<ButtonInput<KeyCode>>,
    mut app_exit: EventWriter<AppExit>,
    mut sim: Single<&mut Simulation>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut particle_query: Query<(&mut Transform, &mut Particle)>,
    window: Single<&Window>,
    mut kb_cmds: Single<&mut KeyboardCommands>,
    mut messages: Single<&mut Messages>,
) {
    // Esc / Q: quit the app
    if kb.pressed(KeyCode::Escape) || kb.pressed(KeyCode::KeyQ) {
        app_exit.send(AppExit::Success);
    }

    // ?: display help
    if kb.just_pressed(KeyCode::Slash) && (kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight)) {
        let mut kb_help = String::new();
        for (key, cmd) in kb_cmds.commands.iter() {
            if !kb_help.is_empty() {
                kb_help.push('\n');
            }
            kb_help.push_str(&format!("{key:?} - {}", cmd.description));
        }
        messages.messages.push(MessageText {
            text: Some(kb_help.into()),
            start_time: Instant::now(),
            duration: Duration::from_secs(5),
        });
    }

    let now = Instant::now();
    let cursor_pos = if let Some(cursor_position) = window.cursor_position() {
        let (camera, camera_transform) = *camera_query;

        // Calculate a world position based on the cursor's position.
        camera
            .viewport_to_world_2d(camera_transform, cursor_position)
            .unwrap_or(Vec2::splat(f32::MAX))
    } else {
        Vec2::splat(f32::MAX)
    };

    for key in kb.get_pressed() {
        if let Some(command) = kb_cmds.commands.get_mut(key) {
            if now.duration_since(command.last_action_time) >= command.interval {
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
