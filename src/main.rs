mod digit_keys;
mod particle;
mod sim;
mod spatial_hash;

use std::time::Instant;

use bevy::color::palettes::basic::YELLOW;
use bevy::prelude::*;
use bevy::window::WindowResized;

use crate::digit_keys::{key_number, DIGIT_KEYS};
use crate::particle::Particle;
use crate::sim::Simulation;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update, velocity_arrows, handle_keypress, on_resize))
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
}

fn update(
    mut particle_query: Query<(&mut Transform, &mut Particle)>,
    time: Res<Time>,
    mut sim: Single<&mut Simulation>,
) {
    let start_time = Instant::now();

    sim.calculate_densities();
    sim.calculate_pressures(time.delta_secs());

    particle_query.iter_mut().for_each(|(mut transform, particle)| {
        if sim.frames_to_advance() > 0 {
            sim.apply_velocity(particle.id);
        }
        transform.translation.x = sim.positions[particle.id].x;
        transform.translation.y = sim.positions[particle.id].y;
    });

    // Dev build, 500 particles:
    //   par_iter_mut: avg 0.00226 sec
    //   iter_mut:     avg 0.00765 sec
    //   So, 3.38 times slower.
    // Release build, 500 particles:
    //   par_iter_mut: avg 0.000281 sec
    //   iter_mut:     avg 0.000822 sec
    //   2.926 times slower.
    // 5000 particles:
    //   release: 0.015326, dev: 0.10116934342857142 --> 6.6 times slower!
    sim.debug(format!("elapsed: {}", Instant::now().duration_since(start_time).as_secs_f32()));

    sim.end_frame();
}

fn velocity_arrows(mut gizmos: Gizmos, particle_query: Query<(&Transform, &Particle)>, sim: Single<&Simulation>) {
    if sim.show_arrows() {
        particle_query.iter().for_each(|(transform, particle)| {
            let arrow_end = transform.translation.xy() + sim.velocities[particle.id] * 10.0;
            gizmos
                .arrow(transform.translation.xy().extend(0.0), arrow_end.extend(0.0), YELLOW)
                .with_tip_length(0.001);
        });
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
    if kb.just_pressed(KeyCode::KeyG) {
        sim.toggle_gravity();
    }
    if kb.just_pressed(KeyCode::KeyL) {
        sim.log_next_frame();
    }
    if kb.just_pressed(KeyCode::KeyR) {
        sim.reset();
    }
    if kb.just_pressed(KeyCode::KeyS) {
        if kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight) {
            sim.inc_smoothing_radius();
        } else {
            sim.dec_smoothing_radius();
        }
    }
    if kb.just_pressed(KeyCode::Escape) {
        app_exit.send(AppExit::Success);
    }
}

/// This system shows how to respond to a window being resized.
/// Whenever the window is resized, the text will update with the new resolution.
fn on_resize(mut resize_reader: EventReader<WindowResized>, mut sim: Single<&mut Simulation>) {
    for e in resize_reader.read() {
        // When resolution is being changed
        sim.on_resize(e.width, e.height);
    }
}
