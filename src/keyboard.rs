use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::MessageText;
use crate::components::*;
use crate::sim_struct::Simulation;

/// Defines a keyboard command to associate with a keypress.
/// Each command can have a different repeat rate.
pub struct KeyboardCommand {
    pub key_text: String,
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
    messages: &mut Single<&mut Notifications>,
);

impl KeyboardCommands {
    pub fn create() -> Self {
        let mut kb_cmds = KeyboardCommands {
            commands: BTreeMap::new(),
        };

        // Space: freeze / unfreeze particle motion.
        kb_cmds.add_command(KeyCode::Space, "Pause", 250, pause);

        // 1: advance 1 frame.
        kb_cmds.add_command(KeyCode::Digit1, "Advance 1 frame", 50, |sim, _, _, _, _| sim.set_frames_to_show(1));

        // A: toggle velocity arrows
        // This was useful during development, when the number of particles is small, but it's of no use with thousands of particles Arrows are just too small, and if I make them
        // bigger, it's just a mess and provides no value.
        // kb_cmds.add_command(KeyCode::KeyA, "Toggle velocity arrows", 250, |sim, _, _, _, _| sim.toggle_arrows());

        // C: toggle display of smoothing radius circle.
        kb_cmds.add_command(KeyCode::KeyC, "Show smoothing radius around a particle", 250, |sim, _, _, _, _| {
            sim.toggle_smoothing_radius()
        });
        // D: toggle between fixed delta and actual frame rate.
        kb_cmds.add_command(KeyCode::KeyF, "Toggle FPS", 500, toggle_fps);
        // F: toggle FPS
        kb_cmds.add_command(KeyCode::KeyD, "Toggle Fixed/Actual delta in calculations", 500, toggle_delta);
        // G: adjust gravity
        kb_cmds.add_command(KeyCode::KeyG, "Decrease gravity (shift: inc)", 50, adj_gravity);
        // H: toggle heat map type (velocity and density).
        kb_cmds.add_command(KeyCode::KeyH, "Toggle heatmap", 500, toggle_heatmap);
        // I: toggle inertia
        kb_cmds.add_command(KeyCode::KeyI, "Reset inertia", 250, reset_inertia);
        // L: log debug info in the next frame
        kb_cmds.add_command(KeyCode::KeyL, "Log debug info", 250, |sim, _, _, _, _| sim.log_next_frame());
        // P: adjust pressure multiplier.
        kb_cmds.add_command(KeyCode::KeyP, "Decrease pressure multiplier (shift: inc)", 100, adj_pressure);
        // O: toggle use of predicted positions
        kb_cmds.add_command(KeyCode::KeyO, "Toggle use of predicted positions", 500, toggle_predicted);
        // R: reset the simulation
        kb_cmds.add_command(KeyCode::KeyR, "Reset particles", 250, |sim, _, _, _, _| sim.reset());
        // S: adjust smoothing radius.
        kb_cmds.add_command(KeyCode::KeyS, "Decrease smoothing radius (shift: inc)", 250, adj_smoothing_radius);
        // V: adjust viscosity strength.
        kb_cmds.add_command(KeyCode::KeyV, "Decrease viscosity (shift: inc)", 50, adj_viscosity);
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
                key_text: key_to_string(key),
                description: description.into(),
                last_action_time: Instant::now(),
                interval: Duration::from_millis(interval_millis),
                action,
            },
        );
    }
}

fn key_to_string(key: KeyCode) -> String {
    match key {
        KeyCode::Digit1 => "1".into(),
        KeyCode::KeyA => "A".into(),
        KeyCode::KeyB => "B".into(),
        KeyCode::KeyC => "C".into(),
        KeyCode::KeyD => "D".into(),
        KeyCode::KeyE => "E".into(),
        KeyCode::KeyF => "F".into(),
        KeyCode::KeyG => "G".into(),
        KeyCode::KeyH => "H".into(),
        KeyCode::KeyI => "I".into(),
        KeyCode::KeyJ => "J".into(),
        KeyCode::KeyK => "K".into(),
        KeyCode::KeyL => "L".into(),
        KeyCode::KeyM => "M".into(),
        KeyCode::KeyN => "N".into(),
        KeyCode::KeyO => "O".into(),
        KeyCode::KeyP => "P".into(),
        KeyCode::KeyQ => "Q".into(),
        KeyCode::KeyR => "R".into(),
        KeyCode::KeyS => "S".into(),
        KeyCode::KeyT => "T".into(),
        KeyCode::KeyU => "U".into(),
        KeyCode::KeyV => "V".into(),
        KeyCode::KeyW => "W".into(),
        KeyCode::KeyX => "X".into(),
        KeyCode::KeyY => "Y".into(),
        KeyCode::KeyZ => "Z".into(),
        KeyCode::Space => "Space".into(),
        other => format!("{:?}", other),
    }
}

fn pause(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    _msgs: &mut Single<&mut Notifications>,
) {
    if sim.frames_to_advance() == 0 {
        sim.set_frames_to_show(u32::MAX);
    } else {
        sim.set_frames_to_show(0);
    }
}

fn toggle_delta(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Notifications>,
) {
    sim.toggle_delta();
    msgs.messages.push(MessageText {
        text: if sim.debug.fixed_delta { "Fixed delta".into() } else { "Actual delta".into() },
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn toggle_fps(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    _msgs: &mut Single<&mut Notifications>,
) {
    sim.toggle_fps();
}

fn adj_gravity(
    sim: &mut Simulation,
    shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Notifications>,
) {
    if shift {
        sim.adj_gravity(true);
    } else {
        sim.adj_gravity(false);
    }
    msgs.messages.push(MessageText {
        text: format!("Gravity: {:.1}", sim.gravity.y / sim.particle_size),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn adj_pressure(
    sim: &mut Simulation,
    shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Notifications>,
) {
    if shift {
        sim.adj_pressure(true);
    } else {
        sim.adj_pressure(false);
    }
    msgs.messages.push(MessageText {
        text: format!("Pressure multiplier: {:.1}", sim.pressure_multiplier / sim.particle_size),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn toggle_heatmap(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Notifications>,
) {
    sim.toggle_heatmap();
    msgs.messages.push(MessageText {
        text: if sim.debug.density_heatmap { "Density heatmap".into() } else { "Velocity heatmap".into() },
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn reset_inertia(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Notifications>,
) {
    sim.reset_inertia();
    msgs.messages.push(MessageText {
        text: "Inertia reset".into(),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn toggle_predicted(
    sim: &mut Simulation,
    _shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Notifications>,
) {
    sim.toggle_predicted();
    msgs.messages.push(MessageText {
        text: format!("Prediction {}", if sim.debug.use_predicted_positions { "on" } else { "off" }),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn adj_smoothing_radius(
    sim: &mut Simulation,
    shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Notifications>,
) {
    let factor = 0.5;
    if shift {
        sim.adj_smoothing_radius(factor);
    } else {
        sim.adj_smoothing_radius(-factor);
    }
    msgs.messages.push(MessageText {
        text: format!("Smoothing radius: {:.2}", sim.smoothing_radius / sim.particle_size),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn adj_viscosity(
    sim: &mut Simulation,
    shift: bool,
    _cursor_pos: &Vec2,
    _particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    msgs: &mut Single<&mut Notifications>,
) {
    if shift {
        sim.adj_viscosity(true);
    } else {
        sim.adj_viscosity(false);
    }
    msgs.messages.push(MessageText {
        text: format!("Viscosity strength: {:.2}", sim.viscosity_strength),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });
}

fn watch_particle(
    sim: &mut Simulation,
    shift: bool,
    cursor_pos: &Vec2,
    particle_query: &mut Query<(&mut Transform, &mut Particle)>,
    _msgs: &mut Single<&mut Notifications>,
) {
    if shift {
        particle_query
            .par_iter_mut()
            .for_each(|(_, mut particle)| particle.watched = false);
    } else {
        particle_query.par_iter_mut().for_each(|(transform, mut particle)| {
            if (transform.translation.xy() - cursor_pos).length() <= sim.particle_size / 2.0 {
                println!(
                    "Watching particle {} @({},{}) density={}, velocity={:?}",
                    particle.id,
                    transform.translation.x,
                    transform.translation.y,
                    sim.densities[particle.id],
                    sim.velocities[particle.id]
                );
                particle.watched = true;
            }
        });
    }
}
