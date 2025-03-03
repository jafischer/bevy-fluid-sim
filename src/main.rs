mod args;
mod particle;
mod sim;
mod spatial_hash;
mod sim_settings;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::GOLD;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResized, WindowResolution};

use crate::args::ARGS;
use crate::particle::Particle;
use crate::sim::Simulation;

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct MessageText {
    pub text: Option<String>,
    pub start_time: Instant,
    pub duration: Duration,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let win_size: Vec<_> = ARGS.win.split(',').collect();
    if win_size.len() != 2 {
        return Err("Incorrect window size".into());
    }
    let width: u16 = win_size[0].parse()?;
    let height: u16 = win_size[1].parse()?;

    App::new()
        .add_plugins((DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(width as f32, height as f32),
                ..default()
            }),
            ..default()
        }),))
        .insert_resource(Time::<Fixed>::from_hz(128.0))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_particles,
                draw_debug_info,
                handle_keypress,
                handle_mouse_clicks,
                on_resize,
                update_fps,
                display_message,
            ),
        )
        .run();

    Ok(())
}

fn setup(mut commands: Commands, window: Single<&Window>) {
    let mut sim = Simulation::new(window.width(), window.height());
    let scale = sim.scale;
    commands.spawn((Camera2d, Transform::from_scale(Vec3::splat(scale))));
    sim.spawn_particles(&mut commands);
    commands.spawn(sim);

    commands
        .spawn((
            // Create a Text with multiple child spans.
            Text::new("FPS: "),
            TextFont {
                font_size: 16.0,
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            (
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(GOLD.into()),
            ),
            FpsText,
        ));

    commands.spawn((
        Text::new(format!("{} particles", ARGS.num)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        // Set the justification of the Text
        TextLayout::new_with_justify(JustifyText::Right),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(5.0),
            ..default()
        },
    ));

    commands.spawn((
        Text2d::default(),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextLayout::new_with_justify(JustifyText::Center),
        Transform::from_scale(Vec3::splat(scale)).with_translation(Vec3::new(0.0, 2.0, 1.0)),
        MessageText {
            text: Some("Press ? for keyboard commands".into()),
            start_time: Instant::now(),
            duration: Duration::from_secs(2),
        },
    ));

    commands.spawn(KeyboardCommands::new());
}

const COLD: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const NEUTRAL: Vec3 = Vec3::new(0.0, 0.0, 0.5);
const HOT: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const COLD_DIFF: Vec3 = Vec3::new(NEUTRAL.x - COLD.x, NEUTRAL.y - COLD.y, NEUTRAL.z - COLD.z);
const WARM_DIFF: Vec3 = Vec3::new(HOT.x - NEUTRAL.x, HOT.y - NEUTRAL.y, HOT.z - NEUTRAL.z);

const STOPPED: Vec3 = Vec3::new(0.0, 0.0, 0.2);
const FAST: Vec3 = Vec3::new(0.5, 0.5, 1.0);
const SPEED_DIFF: Vec3 = Vec3::new(FAST.x - STOPPED.x, FAST.y - STOPPED.y, FAST.z - STOPPED.z);

fn update_particles(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut Particle)>,
    time: Res<Time>,
    mut sim: Single<&mut Simulation>,
) {
    sim.update_particles(time.delta_secs());

    let mut max_speed: f32 = 0.0;

    particle_query.iter_mut().for_each(|(entity, mut transform, particle)| {
        transform.translation.x = sim.positions[particle.id].x;
        transform.translation.y = sim.positions[particle.id].y;
        let color = if particle.watched {
            Color::linear_rgb(1.0, 1.0, 0.0)
        } else if sim.debug.use_heatmap {
            let rgb = if sim.densities[particle.id].0 < sim.target_density {
                let density_scale = sim.densities[particle.id].0 / sim.target_density;
                COLD + density_scale * COLD_DIFF
            } else {
                let density_scale = (sim.densities[particle.id].0 - sim.target_density) / sim.target_density;
                NEUTRAL + density_scale.min(4.0) / 4.0 * WARM_DIFF
            };
            Color::linear_rgb(rgb.x, rgb.y, rgb.z)
        } else {
            let speed = sim.velocities[particle.id].length();
            let speed_scale = speed / (sim.speed_limit * sim.particle_size * time.delta_secs());
            max_speed = max_speed.max(speed_scale);
            let rgb = COLD + speed_scale * SPEED_DIFF;
            Color::linear_rgb(rgb.x, rgb.y, rgb.z)
        };

        commands.entity(entity).insert(Sprite {
            custom_size: Some(Vec2::splat(sim.particle_size * ARGS.sprite_size)),
            color,
            ..Default::default()
        });
    });

    sim.debug(format!("max speed: {max_speed}"));

    sim.end_frame();
}

fn update_fps(mut query: Query<&mut TextSpan, With<FpsText>>, time: Res<Time>) {
    for mut span in &mut query {
        **span = format!("{:.1}", 1.0 / time.delta_secs());
    }
}

fn display_message(mut query: Query<(&mut Text2d, &mut MessageText)>) {
    for (mut text, mut message_text) in &mut query {
        if let Some(msg_text) = message_text.text.as_ref() {
            let duration = Instant::now().duration_since(message_text.start_time);
            if duration > message_text.duration {
                message_text.text = None;
                **text = String::new();
            } else {
                **text = msg_text.to_string();
            }
        }
    }
}

fn draw_debug_info(mut gizmos: Gizmos, particle_query: Query<(&Transform, &Particle)>, sim: Single<&Simulation>) {
    if sim.debug.show_arrows {
        particle_query.iter().for_each(|(transform, particle)| {
            let arrow_end = transform.translation.xy() + sim.velocities[particle.id] * 2.0;
            gizmos
                .arrow(transform.translation.xy().extend(0.0), arrow_end.extend(0.0), YELLOW)
                .with_tip_length(sim.particle_size * 2.0);
        });
    }
    if sim.debug.show_smoothing_radius {
        gizmos.circle_2d(sim.positions[0], sim.smoothing_radius, LIME);
    }
    if sim.debug.show_region_grid {
        let bottom = -sim.half_bounds_size.y;
        let left = -sim.half_bounds_size.x;
        for row in 0..sim.region_rows {
            for col in 0..sim.region_cols {
                gizmos.rect_2d(
                    Vec2::new(left + col as f32 * sim.smoothing_radius, bottom + row as f32 * sim.smoothing_radius),
                    Vec2::splat(sim.smoothing_radius),
                    LIME,
                );
            }
        }
    }
}

struct KeyboardCommand {
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
    message_text: &mut Single<&mut MessageText>,
);

#[derive(Component)]
struct KeyboardCommands {
    pub commands: HashMap<KeyCode, KeyboardCommand>,
}

impl KeyboardCommands {
    fn new() -> Self {
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
        kb_cmds.add_command(KeyCode::KeyG, "Decrease gravity (shift: increase)", 50, |sim, shift, _, _, msg| {
            if shift {
                sim.adj_gravity(-0.5);
            } else {
                sim.adj_gravity(0.5);
            }
            msg.text = Some(format!("Gravity: {:.1}", sim.gravity.y));
            msg.start_time = Instant::now();
            msg.duration = Duration::from_secs(1);
        });
        // H: toggle heat map
        kb_cmds.add_command(KeyCode::KeyH, "Toggle heatmap", 250, |sim, _, _, _, msg| {
            sim.toggle_heatmap();
            if sim.debug.use_heatmap {
                msg.text = Some("Density heatmap".into());
                msg.start_time = Instant::now();
                msg.duration = Duration::from_secs(1);
            } else {
                msg.text = Some("Velocity heatmap".into());
                msg.start_time = Instant::now();
                msg.duration = Duration::from_secs(1);
            }
        });
        // I: toggle inertia (see sim.calculate_pressure()).
        kb_cmds.add_command(KeyCode::KeyI, "Toggle inertia", 250, |sim, _, _, _, msg| {
            sim.toggle_inertia();
            msg.text = Some(format!("Inertia {}", if sim.debug.use_inertia { "on" } else { "off" }));
            msg.start_time = Instant::now();
            msg.duration = Duration::from_secs(1);
        });
        // L: log debug info in the next frame
        kb_cmds.add_command(KeyCode::KeyL, "Log debug info", 250, |sim, _, _, _, _| sim.log_next_frame());
        // R: reset the simulation
        kb_cmds.add_command(KeyCode::KeyR, "Reset particles", 250, |sim, _, _, _, _| sim.reset());
        // I: toggle inertia (see sim.calculate_pressure()).
        kb_cmds.add_command(KeyCode::KeyV, "Toggle inertia", 250, |sim, _, _, _, msg| {
            sim.toggle_viscosity();
            msg.text = Some(format!("Viscosity {}", if sim.debug.use_viscosity { "on" } else { "off" }));
            msg.start_time = Instant::now();
            msg.duration = Duration::from_secs(1);
        });

        // S: increase/decrease smoothing radius.
        kb_cmds.add_command(
            KeyCode::KeyS,
            "Decrease smoothing radius (shift: increase)",
            50,
            |sim, shift, _, _, msg| {
                if shift {
                    sim.adj_smoothing_radius(0.01);
                } else {
                    sim.adj_smoothing_radius(-0.01);
                }
                msg.text = Some(format!("Smoothing radius: {:.2}", sim.smoothing_radius));
                msg.start_time = Instant::now();
                msg.duration = Duration::from_secs(1);
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

    fn add_command(&mut self, key: KeyCode, description: &str, interval_millis: u64, action: KeyboardAction) {
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

fn handle_keypress(
    kb: Res<ButtonInput<KeyCode>>,
    mut app_exit: EventWriter<AppExit>,
    mut sim: Single<&mut Simulation>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut particle_query: Query<(&mut Transform, &mut Particle)>,
    window: Single<&Window>,
    mut kb_cmds: Single<&mut KeyboardCommands>,
    mut message_text: Single<&mut MessageText>,
) {
    // Esc / Q: quit the app
    if kb.pressed(KeyCode::Escape) || kb.pressed(KeyCode::KeyQ) {
        app_exit.send(AppExit::Success);
    }
    
    // ?: display help
    if kb.pressed(KeyCode::Slash) && (kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight)) {    
        let mut kb_help = String::new();
        for (key, cmd) in kb_cmds.commands.iter() {
            if !kb_help.is_empty() {
                kb_help.push('\n');
            }
            kb_help.push_str(&format!("{key:?} - {}", cmd.description));
        }
        message_text.text = Some(kb_help.into());
        message_text.start_time = Instant::now();
        message_text.duration = Duration::from_secs(5);
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
                    &mut message_text,
                );
            }
        }
    }
}

// Handles clicks on the plane that reposition the object.
fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut sim: Single<&mut Simulation>,
    window: Single<&Window>,
) {
    sim.interaction_input_strength = 0.0;

    let left_click = buttons.pressed(MouseButton::Left);
    let right_click = buttons.pressed(MouseButton::Right);
    if !left_click && !right_click {
        return;
    }
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera_query;

    // Calculate a world position based on the cursor's position.
    let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };

    sim.interaction_input_strength = if left_click { -200.0 } else { 200.0 };
    sim.interaction_input_point = point;
}

/// This system shows how to respond to a window being resized.
/// Whenever the window is resized, the text will update with the new resolution.
fn on_resize(mut resize_reader: EventReader<WindowResized>, mut sim: Single<&mut Simulation>) {
    for e in resize_reader.read() {
        // When resolution is being changed
        sim.on_resize(e.width, e.height);
    }
}
