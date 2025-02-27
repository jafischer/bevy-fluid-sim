mod digit_keys;
mod particle;
mod sim;
mod spatial_hash;
mod args;

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::GOLD;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResized, WindowResolution};
use clap::Parser;
use crate::args::ARGS;
use crate::digit_keys::{key_number, DIGIT_KEYS};
use crate::particle::Particle;
use crate::sim::Simulation;

#[derive(Component)]
struct FpsText;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let win_size: Vec<_> = ARGS.win.split(',').collect();
    if win_size.len() != 2 {
        return Err("Incorrect window size".into());
    }
    let width: u16 = win_size[0].parse()?;
    let height: u16 = win_size[1].parse()?;
    
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(width as f32, height as f32),
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_particles, draw_debug_info, handle_keypress, handle_mouse_clicks, on_resize, update_fps),
        )
        .run();
    
    Ok(())
}

fn setup(
    mut commands: Commands,
    window: Single<&Window>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut sim = Simulation::new(window.width(), window.height());

    commands.spawn((Camera2d, Transform::from_scale(Vec3::splat(sim.scale))));
    sim.spawn_particles(&mut commands, &mut meshes, &mut materials);
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
}

fn update_particles(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut Particle)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut sim: Single<&mut Simulation>,
) {
    let cold = Vec3::new(0.0, 0.5, 1.0);
    let hot = Vec3::new(1.0, 0.2, 0.1);
    let diff = hot - cold;

    sim.update_particles(time.delta_secs());

    particle_query.iter_mut().for_each(|(entity, mut transform, particle)| {
        transform.translation.x = sim.positions[particle.id].x;
        transform.translation.y = sim.positions[particle.id].y;
        if particle.watched {
            commands
                .entity(entity)
                .insert(MeshMaterial2d(materials.add(Color::linear_rgb(1.0, 1.0, 0.0))));
        } else {
            let density_scale = (sim.densities[particle.id].0 - sim.target_density) / sim.target_density;
            let color = cold + density_scale.clamp(0.0, 2.0) / 2.0 * diff;
            // if sim.densities[particle.id].0 < 2.0 {
            //     // use gizmo to draw a circle here
            // }
            commands
                .entity(entity)
                .insert(MeshMaterial2d(materials.add(Color::linear_rgb(color.x, color.y, color.z))));
        }
    });

    sim.end_frame();
}

fn update_fps(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut TextSpan, With<FpsText>>) {
    for mut span in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                // Update the value of the second section
                **span = format!("{value:.1}");
            }
        }
    }
}

fn draw_debug_info(mut gizmos: Gizmos, particle_query: Query<(&Transform, &Particle)>, sim: Single<&Simulation>) {
    if sim.debug.show_arrows {
        particle_query.iter().for_each(|(transform, particle)| {
            let arrow_end = transform.translation.xy() + sim.velocities[particle.id] * 5.0;
            gizmos
                .arrow(transform.translation.xy().extend(0.0), arrow_end.extend(0.0), YELLOW)
                .with_tip_length(0.05);
        });
    }
    if sim.debug.show_smoothing_radius {
        let (transform, _) = particle_query.iter().next().unwrap();
        gizmos.circle_2d(transform.translation.xy(), sim.smoothing_radius, LIME);
    }
}

fn handle_keypress(
    kb: Res<ButtonInput<KeyCode>>,
    mut app_exit: EventWriter<AppExit>,
    mut sim: Single<&mut Simulation>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut particle_query: Query<(&mut Transform, &mut Particle)>,
    window: Single<&Window>,
) {
    // Space: freeze / unfreeze particle motion.
    if kb.just_pressed(KeyCode::Space) {
        if sim.frames_to_advance() == 0 {
            sim.set_frames_to_show(u32::MAX);
        } else {
            sim.set_frames_to_show(0);
        }
    }
    // 1-0: advance 1-10 frames.
    if kb.any_just_pressed(DIGIT_KEYS) {
        kb.get_just_pressed().for_each(|key| {
            let count = sim.frames_to_advance() + key_number(key);
            sim.set_frames_to_show(count);
        });
    }
    // A: toggle velocity arrows
    if kb.just_pressed(KeyCode::KeyA) {
        sim.toggle_arrows();
    }
    // C: toggle smoothing radius circle.
    if kb.just_pressed(KeyCode::KeyC) {
        sim.toggle_circles();
    }
    // G: increase/decrease gravity
    if kb.just_pressed(KeyCode::KeyG) {
        if kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight) {
            sim.adj_gravity(-0.5);
        } else {
            sim.adj_gravity(0.5);
        }
    }
    // I: toggle inertia (see sim.calculate_pressure()).
    if kb.just_pressed(KeyCode::KeyI) {
        sim.toggle_inertia();
    }
    // L: log debug info in the next frame
    if kb.just_pressed(KeyCode::KeyL) {
        sim.log_next_frame();
    }
    // R: reset the simulation
    if kb.just_pressed(KeyCode::KeyR) {
        sim.reset();
    }
    // S: increase/decrease smoothing radius.
    if kb.just_pressed(KeyCode::KeyS) {
        if kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight) {
            sim.adj_smoothing_radius(0.01);
        } else {
            sim.adj_smoothing_radius(-0.01);
        }
    }
    // W: "watch" the particle(s) under the cursor (color them yellow).
    // Shift-W: clear all watched particles.
    if kb.just_pressed(KeyCode::KeyW) {
        if kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight) {
            particle_query
                .par_iter_mut()
                .for_each(|(_, mut particle)| particle.watched = false);
        } else {
            let Some(cursor_position) = window.cursor_position() else {
                return;
            };
            let (camera, camera_transform) = *camera_query;

            // Calculate a world position based on the cursor's position.
            let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
                return;
            };

            particle_query.par_iter_mut().for_each(|(transform, mut particle)| {
                if (transform.translation.xy() - point).length() <= sim.particle_size / 2.0 {
                    println!(
                        "Watching particle {} @({},{}) density={}",
                        particle.id, transform.translation.x, transform.translation.y, sim.densities[particle.id].0
                    );
                    particle.watched = true;
                }
            });
        }
    }
    // Esc / Q: quit the app
    if kb.just_pressed(KeyCode::Escape) || kb.just_pressed(KeyCode::KeyQ) {
        app_exit.send(AppExit::Success);
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

    sim.interaction_input_strength = if left_click { -300.0 } else { 300.0 };
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
