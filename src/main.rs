mod particle;
mod sim;

use std::f32::consts::PI;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::time::Instant;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::random;

use crate::particle::Particle;
use crate::sim::Simulation;

const NUM_PARTICLES: u32 = 100;

static FRAMES_TO_SHOW: AtomicU32 = AtomicU32::new(0);
static LOG_STUFF: AtomicBool = AtomicBool::new(true);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins,))
        .add_systems(Startup, setup)
        .add_systems(Update, (update, detect_keypress))
        .run();
}

fn setup(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window = window_query.get_single().unwrap();
    let fluid_h = window.height() * 0.67;
    let (grid_size, cols, rows) = subdivide_into_squares(window.width(), fluid_h, NUM_PARTICLES);
    let particle_size = (grid_size * 0.8).max(20.0);

    let smoothing_radius = particle_size * 5.0;
    let simulation = Simulation {
        smoothing_radius,
        smoothing_derivative_scaling_factor: PI * smoothing_radius.powf(4.0) / 6.0,
        smoothing_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
        target_density: 20.0,
        half_bounds_size: Vec2::new(window.width(), window.height()) / 2.0 - particle_size / 2.0,
    };

    println!("{simulation:?}");

    // let mut x = 0.0;
    // while x <= smoothing_radius {
    //     println!("{:3.2}: {:.4}", x, simulation.smoothing_kernel(x));
    //     // println!("skd({}): {}", x, simulation.smoothing_kernel_derivative(x));
    //     x += smoothing_radius / 50.0;
    // }

    commands.spawn(simulation);

    let color = Color::linear_rgb(0.0, 0.3, 1.0);

    commands.spawn(Camera2d);
    let x_start = -window.width() / 2.0;
    let y_start = -window.height() / 2.0;

    for r in 0..rows {
        for c in 0..cols {
            let x = x_start + (c as f32 + 0.5) * grid_size;
            let y = y_start + window.height() - (r as f32 + 0.5) * grid_size;
            commands.spawn((
                Mesh2d(meshes.add(Circle {
                    radius: particle_size / 2.0,
                })),
                MeshMaterial2d(materials.add(color)),
                Transform::from_translation(Vec3 { x, y, z: 0.0 }),
                Particle {
                    id: (r * cols + c) as usize,
                    position: Vec2 { x, y },
                    velocity: Vec2::new(random::<f32>() * 1.0 - 0.5, random::<f32>() * 1.0 - 0.5) * particle_size / 2.0,
                    density: 0.0,
                },
            ));
        }
    }
}

/// Divides a rectangular region into (roughly) n squares.
///
/// Got it from ChatGPT, but as usual even this straightforward function had errors that
/// I had to fix...
fn subdivide_into_squares(w: f32, h: f32, n: u32) -> (f32, u32, u32) {
    // Step 1: Calculate the target area of each square
    let target_area = (w * h) / n as f32;

    // Step 2: Calculate the side length of each square
    let side_length = target_area.sqrt();

    // Step 3: Calculate the number of columns and rows
    let columns = (w / side_length) as u32;
    let rows = (n as f32 / columns as f32) as u32;

    // Step 4: Adjust the final side length to fit evenly
    let side_length = f32::min(w / columns as f32, h / rows as f32);

    (side_length, columns, rows)
}

fn update(
    mut particle_query: Query<(&mut Transform, &mut Particle)>,
    time: Res<Time>,
    param_query: Query<&Simulation>,
) {
    if FRAMES_TO_SHOW.load(Relaxed) == 0 {
        return;
    }

    FRAMES_TO_SHOW.fetch_sub(1, SeqCst);

    // I'll hopefully figure this out later (can't have both a mutable and non-mutable ref to the
    // same collection), but for now just make a copy of the particles.
    let particle_positions: Vec<Vec2> = particle_query.iter().map(|(_, p)| p.position).collect();
    let sim = param_query.iter().next().unwrap();

    let start_time = Instant::now();
    particle_query.par_iter_mut().for_each(|(_, mut particle)| {
        particle.density = sim.density(&particle, &particle_positions);
    });

    if LOG_STUFF.load(Relaxed) {
        println!(
            "highest initial density:{}",
            particle_query
                .iter()
                .max_by(|(_, p1), (_, p2)| p1.density.total_cmp(&p2.density))
                .unwrap()
                .1
                .density
        );
    }

    let particles: Vec<Particle> = particle_query.iter().map(|(_, p)| p.clone()).collect();
    particle_query.par_iter_mut().for_each(|(mut transform, mut particle)| {
        sim.apply_pressure(&mut particle, &particles, time.delta_secs());

        transform.translation.x = particle.position.x;
        transform.translation.y = particle.position.y;
    });

    if LOG_STUFF.load(Relaxed) {
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
        println!("{}", Instant::now().duration_since(start_time).as_secs_f32());
    }

    LOG_STUFF.store(false, Relaxed);
}

fn detect_keypress(kb: Res<ButtonInput<KeyCode>>, mut app_exit: EventWriter<AppExit>) {
    if kb.just_pressed(KeyCode::Space) {
        if FRAMES_TO_SHOW.load(Relaxed) == 0 {
            FRAMES_TO_SHOW.store(u32::MAX, Relaxed);
        } else {
            FRAMES_TO_SHOW.store(0, Relaxed);
        }
    }
    if kb.just_pressed(KeyCode::Digit1) {
        FRAMES_TO_SHOW.fetch_add(1, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit2) {
        FRAMES_TO_SHOW.fetch_add(2, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit3) {
        FRAMES_TO_SHOW.fetch_add(3, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit4) {
        FRAMES_TO_SHOW.fetch_add(4, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit5) {
        FRAMES_TO_SHOW.fetch_add(5, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit6) {
        FRAMES_TO_SHOW.fetch_add(6, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit7) {
        FRAMES_TO_SHOW.fetch_add(7, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit8) {
        FRAMES_TO_SHOW.fetch_add(8, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit9) {
        FRAMES_TO_SHOW.fetch_add(9, Relaxed);
    }
    if kb.just_pressed(KeyCode::Digit0) {
        FRAMES_TO_SHOW.fetch_add(10, Relaxed);
    }
    if kb.just_pressed(KeyCode::KeyL) {
        LOG_STUFF.store(true, Relaxed);
    }
    if kb.just_pressed(KeyCode::Escape) {
        app_exit.send(AppExit::Success);
    }
}
