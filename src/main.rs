mod particle;
mod sim;

use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::time::Instant;

use crate::particle::Particle;
use crate::sim::Simulation;
use bevy::color::palettes::basic::YELLOW;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const NUM_PARTICLES: u32 = 1000;

static FRAMES_TO_SHOW: AtomicU32 = AtomicU32::new(1);
static LOG_STUFF: AtomicBool = AtomicBool::new(true);
static SHOW_ARROWS: AtomicBool = AtomicBool::new(false);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_particles, velocity_arrows, detect_keypress))
        .run();
}

fn setup(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    // window: Single<&Window>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window = window_query.get_single().unwrap();
    let fluid_h = window.height() * 0.67;
    let (grid_size, cols, rows) = subdivide_into_squares(window.width(), fluid_h, NUM_PARTICLES);

    // Because the kernel math blows up with smoothing radius values > 1, we don't want to use the
    // actual window coordinates. In Sebastian's video, at 5:40, he shows a smoothing radius of 0.5
    // that is about 12 particles wide:
    // (https://youtu.be/rSKMYc1CQHE?si=3sibErk0e4CYC5wF&t=340)
    // So we want the grid size to be scaled down to 0.08333 (1/12).
    // grid_size * scale = 0.08333
    // scale = 0.08333 / grid_size
    let scale = 0.08333 / grid_size;
    let grid_size = grid_size * scale;

    let particle_size = grid_size * 0.5;

    commands.spawn(Simulation::new(window.width() * scale, window.height() * scale, particle_size, scale));

    let color = Color::linear_rgb(0.0, 0.3, 1.0);

    commands.spawn((
        Camera2d,
        Transform::from_scale(Vec3::splat(scale)),
    ));
    let scaled_width = window.width() * scale;
    let scaled_height = window.height() * scale;

    let x_start = -scaled_width / 2.0;
    let y_start = -scaled_height / 2.0;
    for r in 0..rows {
        for c in 0..cols {
            let x = x_start + (c as f32 + 0.5) * grid_size;
            let y = y_start + scaled_height - (r as f32 + 0.5) * grid_size;
            // let velocity = Vec2::new(random::<f32>() * 1.0 - 0.5, random::<f32>() * 1.0 - 0.5) * particle_size / 2.0;
            let velocity = Vec2::ZERO;
            commands.spawn((
                Mesh2d(meshes.add(Circle {
                    radius: particle_size / 2.0,
                })),
                MeshMaterial2d(materials.add(color)),
                Transform::from_translation(Vec3 { x, y, z: 0.0 }),
                Particle {
                    id: (r * cols + c) as usize,
                    position: Vec2 { x, y },
                    velocity,
                    density: 0.0,
                },
            ));
        }
    }
}

fn move_particles(
    mut particle_query: Query<(&mut Transform, &mut Particle)>,
    time: Res<Time>,
    sim: Single<&Simulation>,
) {
    if FRAMES_TO_SHOW.load(Relaxed) == 0 {
        return;
    }

    FRAMES_TO_SHOW.fetch_sub(1, SeqCst);

    // I'll hopefully figure this out later (can't have both a mutable and non-mutable ref to the
    // same collection), but for now just make a copy of the particles.
    let particle_positions: Vec<Vec2> = particle_query.iter().map(|(_, p)| p.position).collect();

    let start_time = Instant::now();
    particle_query.par_iter_mut().for_each(|(_, mut particle)| {
        particle.density = sim.density(&particle, &particle_positions);
    });

    if LOG_STUFF.load(Relaxed) {
        println!(
            "highest density:{}",
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
        println!("elapsed: {}", Instant::now().duration_since(start_time).as_secs_f32());
    }

    LOG_STUFF.store(false, Relaxed);
}

fn velocity_arrows(mut gizmos: Gizmos, mut particle_query: Query<(&Transform, &mut Particle)>) {
    if SHOW_ARROWS.load(Relaxed) {
        particle_query.iter_mut().for_each(|(transform, particle)| {
            let arrow_end = transform.translation.xy() + particle.velocity * 10.0;
            gizmos
                .arrow(transform.translation.xy().extend(0.0), arrow_end.extend(0.0), YELLOW)
                .with_tip_length(0.001);
        });
    }
}

const DIGIT_KEYS: [KeyCode; 20] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::Digit0,
    KeyCode::Numpad1,
    KeyCode::Numpad2,
    KeyCode::Numpad3,
    KeyCode::Numpad4,
    KeyCode::Numpad5,
    KeyCode::Numpad6,
    KeyCode::Numpad7,
    KeyCode::Numpad8,
    KeyCode::Numpad9,
    KeyCode::Numpad0,
];

fn detect_keypress(kb: Res<ButtonInput<KeyCode>>, mut app_exit: EventWriter<AppExit>) {
    if kb.just_pressed(KeyCode::Space) {
        if FRAMES_TO_SHOW.load(Relaxed) == 0 {
            FRAMES_TO_SHOW.store(u32::MAX, Relaxed);
        } else {
            FRAMES_TO_SHOW.store(0, Relaxed);
        }
    }
    if kb.any_just_pressed(DIGIT_KEYS) {
        kb.get_just_pressed().for_each(|key| {
            let count = match key {
                KeyCode::Digit1 | KeyCode::Numpad1 => 1,
                KeyCode::Digit2 | KeyCode::Numpad2 => 2,
                KeyCode::Digit3 | KeyCode::Numpad3 => 3,
                KeyCode::Digit4 | KeyCode::Numpad4 => 4,
                KeyCode::Digit5 | KeyCode::Numpad5 => 5,
                KeyCode::Digit6 | KeyCode::Numpad6 => 6,
                KeyCode::Digit7 | KeyCode::Numpad7 => 7,
                KeyCode::Digit8 | KeyCode::Numpad8 => 8,
                KeyCode::Digit9 | KeyCode::Numpad9 => 9,
                KeyCode::Digit0 | KeyCode::Numpad0 => 10,
                _ => 0,
            };
            println!("{count}");
            FRAMES_TO_SHOW.fetch_add(count, Relaxed);
        });
    }
    if kb.just_pressed(KeyCode::KeyL) {
        LOG_STUFF.store(true, Relaxed);
    }
    if kb.just_pressed(KeyCode::KeyA) {
        SHOW_ARROWS.store(!SHOW_ARROWS.load(Relaxed), Relaxed);
    }
    if kb.just_pressed(KeyCode::Escape) {
        app_exit.send(AppExit::Success);
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
    let columns = w / side_length;
    let rows = n as f32 / columns;

    // Step 4: Adjust the final side length to fit evenly
    let side_length = f32::min(w / columns, h / rows);

    (side_length, columns as u32, rows as u32)
}
