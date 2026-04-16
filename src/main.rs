mod args;
mod keyboard;
mod messages;
mod particle;
mod sim_impl;
mod sim_settings;
mod sim_struct;

use std::ops::{Deref, DerefMut};
use std::sync::Mutex;

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::GOLD;
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow, WindowResized, WindowResolution};
use clap::Parser;
use once_cell::sync::Lazy;

use crate::args::Args;
use crate::keyboard::{handle_keypress, KeyboardCommands};
use crate::messages::{display_messages, spawn_messages, MessageText, Messages};
use crate::particle::Particle;
use crate::sim_struct::Simulation;

#[derive(Component)]
struct FpsText;

static ARGS: Lazy<Args> = Lazy::new(Args::parse);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let win_size: Vec<_> = ARGS.win.split(',').collect();
    if win_size.len() != 2 {
        return Err("Incorrect window size".into());
    }
    let width: u32 = win_size[0].parse()?;
    let height: u32 = win_size[1].parse()?;

    App::new()
        // Background color
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 0.05)))
        .add_plugins((DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(width, height),
                ..default()
            }),
            ..default()
        }),))
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
    let mut sim = Simulation::new(
        window.width(),
        window.height(),
        &ARGS,
    );
    commands.spawn(Camera2d);
    sim.spawn_particles(&mut commands);
    commands.spawn(sim);

    // FPS display.
    commands.spawn((
        Text::default(),
        (
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(GOLD.into()),
        ),
        FpsText,
    ));

    // Display the number of particles.
    commands.spawn((
        Text::new(format!("{} particles", ARGS.num_particles)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        // Set the justification of the Text
        TextLayout::new_with_justify(Justify::Right),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(5.0),
            ..default()
        },
    ));

    spawn_messages(&mut commands);

    // Keyboard commands component
    commands.spawn(KeyboardCommands::create());
}

// Some color definitions for blending.
const COLD: Vec3 = Vec3::new(0.0, 0.0, 0.5);
const HOT: Vec3 = Vec3::new(1.0, 0.0, 0.5);

const STOPPED: Vec3 = Vec3::new(0.1, 0.1, 0.5);
const FAST: Vec3 = Vec3::new(0.8, 1.0, 0.0);

fn update_particles(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut Particle)>,
    time: Res<Time>,
    mut sim: Single<&mut Simulation>,
) {
    sim.update_particles(time.delta_secs());

    particle_query.iter_mut().for_each(|(entity, mut transform, particle)| {
        transform.translation.x = sim.positions[particle.id].x;
        transform.translation.y = sim.positions[particle.id].y;
        let color = if particle.watched {
            Color::linear_rgb(1.0, 1.0, 0.0)
        } else if sim.debug.use_heatmap {
            let density_ratio = (sim.densities[particle.id] - sim.min_density) / sim.max_density;
            let density_scale = density_ratio.powf(2.0);
            let rgb = COLD + density_scale * (HOT - COLD);
            Color::linear_rgb(rgb.x, rgb.y, rgb.z)
        } else {
            let speed_ratio = sim.velocities[particle.id].length() / sim.max_velocity;
            let speed_scale = speed_ratio.powf(0.25);
            let rgb = STOPPED + speed_scale * (FAST - STOPPED);
            Color::linear_rgb(rgb.x, rgb.y, rgb.z)
        };

        commands.entity(entity).insert(Sprite {
            custom_size: Some(Vec2::splat(sim.particle_size * ARGS.sprite_size)),
            color,
            ..Default::default()
        });
    });

    sim.end_frame();
}

static TOT_FPS: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.0));

fn update_fps(mut query: Query<(&mut Text, &FpsText)>, time: Res<Time>, sim: Single<&Simulation>) {
    for (mut span, _) in &mut query {
        if time.delta_secs() == 0.0 {
            return;
        }

        let cur_fps = 1.0 / time.delta_secs();
        let mut tot_fps = TOT_FPS.lock().unwrap();

        *tot_fps.deref_mut() += cur_fps;
        if sim.debug.show_fps {
            **span = format!("FPS: {:5.1} / avg {:.1}", cur_fps, tot_fps.deref() / (sim.debug.current_frame as f32));
        } else if !span.is_empty() {
            **span = String::new();
        }
    }
}

fn draw_debug_info(
    mut gizmos: Gizmos,
    sim: Single<&Simulation>,
    particle_query: Query<(&mut Transform, &mut Particle)>,
    time: Res<Time>,
) {
    if sim.debug.show_arrows {
        particle_query.iter().for_each(|(transform, particle)| {
            let arrow_end = transform.translation.xy() + sim.velocities[particle.id] * time.delta_secs();
            gizmos
                .arrow(transform.translation.xy().extend(0.0), arrow_end.extend(0.0), YELLOW)
                .with_tip_length(sim.particle_size);
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
                    GRAY,
                );
            }
        }
    }
}

// Handles clicks on the plane that reposition the object.
fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    mut sim: Single<&mut Simulation>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras_query: Query<(&Camera, &GlobalTransform)>,
) {
    if let Ok(window) = windows.single() {
        sim.interaction_input_strength = 0.0;

        let left_click = buttons.pressed(MouseButton::Left);
        let right_click = buttons.pressed(MouseButton::Right);
        if !left_click && !right_click {
            return;
        }
        let Some(cursor_position) = window.cursor_position() else {
            return;
        };
        let Some((camera, camera_transform)) = cameras_query.iter().next() else {
            return;
        };

        // Calculate a world position based on the cursor's position.
        let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
            return;
        };

        sim.interaction_input_strength = ARGS.interaction_input_strength * if left_click { 1.0 } else { -1.0 };
        sim.interaction_input_point = point;
    }
}

fn on_resize(mut resize_reader: MessageReader<WindowResized>, mut sim: Single<&mut Simulation>) {
    for e in resize_reader.read() {
        // When resolution is being changed
        sim.on_resize(e.width, e.height);
    }
}
