use std::collections::HashMap;
use std::time::{Duration, Instant};

use bevy::app::AppExit;
use bevy::input::ButtonInput;
use bevy::prelude::*;

use crate::particle::Particle;
use crate::sim::Simulation;
use crate::{MessageText, Messages};

/// Defines a keyboard command to associate with a keypress.
/// Each command can have a different repeat rate.
pub struct KeyboardCommand {
    pub description: String,
    pub last_action_time: Instant,
    pub interval: Duration,
    pub action: KeyboardAction,
}

/// The function that invokes the keyboard action.
/// The parameters are a mishmash of things I just happened to need in the various actions.
type KeyboardAction = fn(
    sim: &mut Simulation,
    // true == shift is pressed
    shift: bool,
    // current mouse cursor position
    cursor_pos: &Vec2,
    particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    messages: &mut Single<&mut Messages>,
);

/// Contains the collection of keyboard commands.
#[derive(Component)]
pub struct KeyboardCommands {
    pub commands: HashMap<KeyCode, KeyboardCommand>,
}

impl KeyboardCommands {
    pub fn create() -> Self {
        let mut kb_cmds = KeyboardCommands {
            commands: HashMap::new(),
        };

        // Space: freeze / unfreeze particle motion.
        kb_cmds.add_command(KeyCode::Space, "Pause", 250, pause);

        // 1: advance 1 frames.
        kb_cmds.add_command(KeyCode::Digit1, "Advance 1 frame", 500, |sim, _, _, _, _| sim.set_frames_to_show(1));
        // A: toggle velocity arrows
        // kb_cmds.add_command(KeyCode::KeyA, "Toggle velocity arrows", 250, |sim, _, _, _, _| sim.toggle_arrows());
        // C: toggle display of smoothing radius circle.
        kb_cmds.add_command(KeyCode::KeyC, "Show smoothing radius around particle 0", 250, |sim, _, _, _, _| {
            sim.toggle_smoothing_radius()
        });
        // G: increase/decrease gravity
        kb_cmds.add_command(KeyCode::KeyG, "Decrease gravity (shift: increase)", 50, adj_gravity);
        // H: toggle heat map
        kb_cmds.add_command(KeyCode::KeyH, "Toggle heatmap", 250, toggle_heatmap);
        // I: toggle inertia
        kb_cmds.add_command(KeyCode::KeyI, "Reset inertia (shift: toggle inertia)", 250, toggle_inertia);
        // L: log debug info in the next frame
        kb_cmds.add_command(KeyCode::KeyL, "Log debug info", 250, |sim, _, _, _, _| sim.log_next_frame());
        // R: reset the simulation
        kb_cmds.add_command(KeyCode::KeyR, "Reset particles", 250, |sim, _, _, _, _| sim.reset());
        // V: toggle viscosity
        kb_cmds.add_command(KeyCode::KeyV, "Toggle inertia", 250, toggle_viscosity);
        // S: increase/decrease smoothing radius.
        kb_cmds.add_command(KeyCode::KeyS, "Decrease smoothing radius (shift: increase)", 50, adj_smoothing_radius);
        // W: "watch" the particle(s) under the cursor (color them yellow).
        // Shift-W: clear all watched particles.
        kb_cmds.add_command(KeyCode::KeyW, "Watch (highlight) particle under cursor", 250, watch_particle);
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

fn pause(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    _msgs: &mut Single<&mut Messages>,
) {
    if sim.frames_to_advance() == 0 {
        sim.set_frames_to_show(u32::MAX);
    } else {
        sim.set_frames_to_show(0);
    }
}

fn adj_gravity(
    sim: &mut Simulation,
    shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Messages>,
) {
    if shift {
        sim.adj_gravity(-0.5);
    } else {
        sim.adj_gravity(0.5);
    }
    msgs.messages.push(MessageText {
        text: format!("Gravity: {:.1}", sim.gravity.y),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn toggle_heatmap(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Messages>,
) {
    sim.toggle_heatmap();
    if sim.debug.use_heatmap {
        msgs.messages.push(MessageText {
            text: "Density heatmap".into(),
            start_time: Instant::now(),
            duration: Duration::from_secs(1),
        });
    } else {
        msgs.messages.push(MessageText {
            text: "Velocity heatmap".into(),
            start_time: Instant::now(),
            duration: Duration::from_secs(1),
        });
    }
}

fn toggle_inertia(
    sim: &mut Simulation,
    shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Messages>,
) {
    if shift {
        sim.toggle_inertia();
        msgs.messages.push(MessageText {
            text: format!("Inertia {}", if sim.debug.use_inertia { "on" } else { "off" }),
            start_time: Instant::now(),
            duration: Duration::from_secs(1),
        });
    } else {
        sim.reset_inertia();
        msgs.messages.push(MessageText {
            text: "Inertia reset".into(),
            start_time: Instant::now(),
            duration: Duration::from_secs(1),
        });
    }
}

fn toggle_viscosity(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Messages>,
) {
    sim.toggle_viscosity();
    msgs.messages.push(MessageText {
        text: format!("Viscosity {}", if sim.debug.use_viscosity { "on" } else { "off" }),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn adj_smoothing_radius(
    sim: &mut Simulation,
    shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Messages>,
) {
    if shift {
        sim.adj_smoothing_radius(0.01);
    } else {
        sim.adj_smoothing_radius(-0.01);
    }
    msgs.messages.push(MessageText {
        text: format!("Smoothing radius: {:.2}", sim.smoothing_radius),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn watch_particle(
    sim: &mut Simulation,
    shift: bool,
    cursor_pos: &Vec2,
    particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    _msgs: &mut Single<&mut Messages>,
) {
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
        let mut kb_help: String = "Keyboard commands:".into();
        // Are we already displaying it?
        for message in &messages.messages {
            if message.text.starts_with(&kb_help) {
                return;
            }
        }
        
        for (key, cmd) in kb_cmds.commands.iter() {
            kb_help.push('\n');
            kb_help.push_str(&format!("{key:?} - {}", cmd.description));
        }
        messages.messages.push(MessageText {
            text: kb_help,
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
