mod digit_keys;
mod particle;
mod sim;
mod spatial_hash;

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::GOLD;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::WindowResized;

use crate::digit_keys::{key_number, DIGIT_KEYS};
use crate::particle::Particle;
use crate::sim::Simulation;

#[derive(Component)]
struct FpsText;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (update, draw_debug_info, handle_keypress, handle_mouse_clicks, on_resize, update_fps))
        .run();
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
                font_size: 14.0,
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            (
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(GOLD.into()),
            ),
            FpsText,
        ));
}

fn update(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut Particle)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut sim: Single<&mut Simulation>,
) {
    sim.calculate_densities();
    sim.calculate_pressures(time.delta_secs());
    let cold = Vec3::new(0.0, 0.2, 1.0);
    let hot = Vec3::new(1.0, 0.2, 0.1);
    let diff = hot - cold;

    if sim.frames_to_advance() > 0 {
        sim.apply_velocities();
    }

    particle_query.iter_mut().for_each(|(entity, mut transform, particle)| {
        transform.translation.x = sim.positions[particle.id].x;
        transform.translation.y = sim.positions[particle.id].y;
        if particle.watched {
            commands
                .entity(entity)
                .insert(MeshMaterial2d(materials.add(Color::linear_rgb(1.0, 1.0, 0.0))));
        } else {
            let density_scale = sim.densities[particle.id] / sim.target_density;
            let color = cold + density_scale.clamp(0.0, 1.0) * diff;
            if sim.densities[particle.id] < 2.0 {
                commands
                    .entity(entity)
                    .insert(MeshMaterial2d(materials.add(Color::linear_rgb(1.0, 1.0, 1.0))));
            } else {
                commands
                    .entity(entity)
                    .insert(MeshMaterial2d(materials.add(Color::linear_rgb(color.x, color.y, color.z))));
            }
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
            let arrow_end = transform.translation.xy() + sim.velocities[particle.id];
            gizmos
                .arrow(transform.translation.xy().extend(0.0), arrow_end.extend(0.0), YELLOW)
                .with_tip_length(0.01);
        });
    }
    if sim.debug.show_circles {
        for (i, (transform, _)) in particle_query.iter().enumerate() {
            if (i % 100) == 0 {
                gizmos.circle_2d(transform.translation.xy(), sim.smoothing_radius, LIME);
            }
        }
    }
}

fn handle_keypress(
    kb: Res<ButtonInput<KeyCode>>,
    mut app_exit: EventWriter<AppExit>,
    mut sim: Single<&mut Simulation>,
) {
    if kb.just_pressed(KeyCode::Space) {
        if sim.frames_to_advance() == 0 {
            sim.set_frames_to_show(u32::MAX);
        } else {
            sim.set_frames_to_show(0);
        }
    }
    if kb.any_just_pressed(DIGIT_KEYS) {
        kb.get_just_pressed().for_each(|key| {
            let count = sim.frames_to_advance() + key_number(key);
            sim.set_frames_to_show(count);
        });
    }
    if kb.just_pressed(KeyCode::KeyA) {
        sim.toggle_arrows();
    }
    if kb.just_pressed(KeyCode::KeyC) {
        sim.toggle_circles();
    }
    if kb.just_pressed(KeyCode::KeyG) {
        if kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight) {
            sim.adj_gravity(-0.5);
        } else {
            sim.adj_gravity(0.5);
        }
    }
    if kb.just_pressed(KeyCode::KeyV) {
        sim.toggle_inc_velocity();
    }
    if kb.just_pressed(KeyCode::KeyL) {
        sim.log_next_frame();
    }
    if kb.just_pressed(KeyCode::KeyR) {
        sim.reset();
    }
    if kb.just_pressed(KeyCode::KeyS) {
        if kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight) {
            sim.adj_smoothing_radius(0.01);
        } else {
            sim.adj_smoothing_radius(-0.01);
        }
    }
    if kb.just_pressed(KeyCode::Escape) {
        app_exit.send(AppExit::Success);
    }
}

// Handles clicks on the plane that reposition the object.
fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    // windows: Query<&Window, With<PrimaryWindow>>,
    window: Single<&Window>,
    // cameras: Query<(&Camera, &GlobalTransform)>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut particle_query: Query<(&mut Transform, &mut Particle)>,
    sim: Single<&Simulation>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
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

    particle_query.par_iter_mut().for_each(|(transform, mut particle)| {
        if (transform.translation.xy() - point).length() <= sim.particle_size / 2.0 {
            println!("Watching particle {} @{:?}", particle.id, transform.translation.xy());
            particle.watched = true;
        }
    });
}

/// This system shows how to respond to a window being resized.
/// Whenever the window is resized, the text will update with the new resolution.
fn on_resize(mut resize_reader: EventReader<WindowResized>, mut sim: Single<&mut Simulation>) {
    for e in resize_reader.read() {
        // When resolution is being changed
        sim.on_resize(e.width, e.height);
    }
}
