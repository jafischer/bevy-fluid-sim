mod args;
mod keyboard;
mod particle;
mod sim;
mod sim_settings;
mod spatial_hash;

use std::time::{Duration, Instant};

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::GOLD;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResized, WindowResolution};

use crate::args::ARGS;
use crate::keyboard::{handle_keypress, KeyboardCommands};
use crate::particle::Particle;
use crate::sim::Simulation;

#[derive(Component)]
struct FpsText;

struct MessageText {
    pub text: Option<String>,
    pub start_time: Instant,
    pub duration: Duration,
}

#[derive(Component)]
struct Messages {
    pub messages: Vec<MessageText>,
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
                display_messages,
            ),
        )
        .run();

    Ok(())
}

fn setup(mut commands: Commands, window: Single<&Window>) {
    // Create the simulation and add it to ECS.
    let mut sim = Simulation::new(window.width(), window.height());
    let scale = sim.scale;
    commands.spawn((Camera2d, Transform::from_scale(Vec3::splat(scale))));
    sim.spawn_particles(&mut commands);
    commands.spawn(sim);

    // FPS display.
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

    // Show the number of particles.
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

    let mut messages = Messages { messages: vec![] };
    messages.messages.push(MessageText {
        text: Some("Left-click to attract, right-click to repel\n\nPress ? for keyboard commands".into()),
        start_time: Instant::now(),
        duration: Duration::from_secs(1),
    });

    // Dynamic message text
    commands.spawn((
        Text2d::default(),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextLayout::new_with_justify(JustifyText::Center),
        Transform::from_scale(Vec3::splat(scale)).with_translation(Vec3::new(0.0, 2.0, 1.0)),
        messages,
    ));

    // Keyboard input
    commands.spawn(KeyboardCommands::create());
}

const COLD: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const NEUTRAL: Vec3 = Vec3::new(0.0, 0.0, 0.5);
const HOT: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const COLD_DIFF: Vec3 = Vec3::new(NEUTRAL.x - COLD.x, NEUTRAL.y - COLD.y, NEUTRAL.z - COLD.z);
const WARM_DIFF: Vec3 = Vec3::new(HOT.x - NEUTRAL.x, HOT.y - NEUTRAL.y, HOT.z - NEUTRAL.z);

const STOPPED: Vec3 = Vec3::new(0.0, 0.0, 0.2);
const FAST: Vec3 = Vec3::new(0.5, 0.5, 1.0);
const SPEED_DIFF: Vec3 = Vec3::new(FAST.x - STOPPED.x, FAST.y - STOPPED.y, FAST.z - STOPPED.z);

const EMPTY: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const FULL: Vec3 = Vec3::new(1.0, 1.0, 1.0);

fn update_particles(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut Particle)>,
    time: Res<Time>,
    mut sim: Single<&mut Simulation>,
    mut gizmos: Gizmos,
) {
    sim.update_particles(time.delta_secs());

    let mut max_speed: f32 = 0.0;
    let mut region_drawn: Vec<bool> = vec![false; sim.region_cols * sim.region_rows];
    let bottom = -sim.half_bounds_size.y;
    let left = -sim.half_bounds_size.x;
    let max_region_count = sim
        .regions
        .iter()
        .map(|row| row.iter().map(|cell| cell.len()).max().unwrap_or(0))
        .max()
        .unwrap_or(0) as f32;
    if sim.debug.show_region_grid {
        for row in 0..sim.region_rows {
            for col in 0..sim.region_cols {
                gizmos.rect_2d(
                    Vec2::new(left + col as f32 * sim.smoothing_radius, bottom + row as f32 * sim.smoothing_radius),
                    Vec2::splat(sim.smoothing_radius),
                    GRAY,
                );
            }
        }
        sim.debug(format!("max_region_count: {max_region_count}"));
    }

    particle_query.iter_mut().for_each(|(entity, mut transform, particle)| {
        if sim.debug.show_region_grid {
            let row = ((sim.positions[particle.id].y - bottom) / sim.smoothing_radius) as usize;
            let col = ((sim.positions[particle.id].x - left) / sim.smoothing_radius) as usize;
            if !region_drawn[row * sim.region_cols + col] {
                transform.translation.x = left + col as f32 * sim.smoothing_radius;
                transform.translation.y = bottom + row as f32 * sim.smoothing_radius;
                let fullness = (sim.regions[row][col].iter().len() as f32 / 100.0).min(1.0);
                let color = Color::linear_rgb(fullness, fullness, fullness);
                commands.entity(entity).insert(Sprite {
                    custom_size: Some(Vec2::splat(sim.smoothing_radius)),
                    color,
                    ..Default::default()
                });
            } else {
                // Move the particle off-screen.
                transform.translation.x = left - sim.smoothing_radius * 2.0;
            }
        } else {
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
        }
    });
    if sim.debug.use_heatmap {
        sim.debug(format!("max speed: {max_speed}"));
    }

    sim.end_frame();
}

fn update_fps(mut query: Query<&mut TextSpan, With<FpsText>>, time: Res<Time>) {
    for mut span in &mut query {
        **span = format!("{:.1}", 1.0 / time.delta_secs());
    }
}

fn display_messages(mut query: Query<(&mut Text2d, &mut Messages)>) {
    for (mut text, mut messages) in &mut query {
        let message_lines: Vec<String> = messages
            .messages
            .iter()
            .filter_map(|message_text| {
                // if let Some(msg_text) = message_text.text.as_ref() {
                let duration = Instant::now().duration_since(message_text.start_time);
                if duration < message_text.duration {
                    Some(message_text.text.clone())
                } else {
                    None
                }
            })
            .map(|s| s.unwrap())
            .collect();
        **text = message_lines.join("\n");
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

    sim.interaction_input_strength = ARGS.interaction_input_strength * if left_click { 1.0 } else { -1.0 };
    sim.interaction_input_point = point;
}

fn on_resize(mut resize_reader: EventReader<WindowResized>, mut sim: Single<&mut Simulation>) {
    for e in resize_reader.read() {
        // When resolution is being changed
        sim.on_resize(e.width, e.height);
    }
}
