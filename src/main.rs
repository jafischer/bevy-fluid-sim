use std::f32::consts::PI;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::random;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Instant;

const NUM_PARTICLES: u32 = 100;
const GRAVITY: f32 = -9.8;
const PARTICLE_MASS: f32 = 1.0;
const PRESSURE_MULTIPLIER: f32 = 200.0;
const COLLISION_DAMPING: f32 = 0.5;

static FREEZE: AtomicBool = AtomicBool::new(true);
static LOG_STUFF: AtomicBool = AtomicBool::new(true);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins,))
        .add_systems(Startup, setup)
        .add_systems(Update, (update, detect_keypress))
        .run();
}

#[derive(Component, Clone, Debug)]
struct Particle {
    id: usize,
    position: Vec2,
    velocity: Vec2,
    density: f32,
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

    let smoothing_radius = particle_size * 10.0;
    let simulation = Simulation {
        smoothing_radius,
        smoothing_derivative_scaling_factor: PI * smoothing_radius.powf(4.0) / 6.0,
        smoothing_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
        target_density: 5069424500000000.0,
        half_bounds_size: Vec2::new(window.width(), window.height()) / 2.0 - particle_size / 2.0,
    };
    
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

fn update(mut particle_query: Query<(&mut Transform, &mut Particle)>, time: Res<Time>,
          param_query: Query<&Simulation>) {
    // I'll hopefully figure this out later, but for now just make a copy of the particles.
    let particles: Vec<Particle> = particle_query.iter().map(|(_, p)| p.clone()).collect();
    let sim = param_query.iter().next().unwrap();

    if LOG_STUFF.load(Relaxed) {
        println!("target density calculation: {}", 2.0 * sim.density(&particles[particles.len() / 2], &particles));
    }

    if !FREEZE.load(Relaxed) {
        let start_time = Instant::now();
        particle_query.par_iter_mut().for_each(|(_, mut particle)| {
            particle.density = sim.density(&particle, &particles);
        });

        particle_query.par_iter_mut().for_each(|(mut transform, mut particle)| {
            let is_first_particle = particle.position == particles[0].position;
            let pressure_force = sim.pressure_force(&particle, &particles);
            let pressure_accel = pressure_force / particle.density;
            let velocity = particle.velocity + (GRAVITY + pressure_accel) * time.delta_secs();
            if is_first_particle && LOG_STUFF.load(Relaxed) {
                println!(
                    "pressure_force: {pressure_force:?} pressure_accel: {pressure_accel:?} velocity: {:?}->{velocity:?}",
                    particle.velocity
                );
            }

            // let velocity = particle.velocity;
            let position = particle.position + velocity;
            (particle.position, particle.velocity) = sim.resolve_collisions(position, velocity);

            transform.translation.x = particle.position.x;
            transform.translation.y = particle.position.y;
        });

        if LOG_STUFF.load(Relaxed) {
            // par_iter_mut: avg 0.00226 sec
            // iter_mut:     avg 0.00765 sec
            // So, 3.38 times slower with 500 particles.
            // Release build:
            // par_iter_mut: avg 0.000281 sec
            // iter_mut:     avg 0.000822 sec
            // 2.926 times slower.
            // 5000 particles:
            // 0.015326 vs 0.10116934342857142, 6.6 times slower!
            println!("{}", Instant::now().duration_since(start_time).as_secs_f32());
        }
    }
    
    LOG_STUFF.store(false, Relaxed);
}

fn detect_keypress(kb: Res<ButtonInput<KeyCode>>) {
    if kb.just_pressed(KeyCode::Space) {
        FREEZE.store(!FREEZE.load(Relaxed), Relaxed);
    }
    if kb.just_pressed(KeyCode::KeyL) {
        LOG_STUFF.store(true, Relaxed);
    }
}

#[derive(Component, Clone, Debug)]
struct Simulation {
    pub smoothing_radius: f32,
    pub smoothing_derivative_scaling_factor: f32,
    pub smoothing_scaling_factor: f32,
    pub target_density: f32,
    pub half_bounds_size: Vec2,
}

impl Simulation {
    pub fn density(&self, pt: &Particle, particles: &Vec<Particle>) -> f32 {
        let mut density = 0.0;
        let is_first_particle = pt.id == 0;

        for (i, particle) in particles.iter().enumerate() {
            if i == pt.id { continue; }
            let distance = (particle.position - pt.position).length().max(0.001);
            let influence = self.smoothing_kernel(distance);
            if is_first_particle && LOG_STUFF.load(Relaxed) {
                println!("distance: {distance:.3} influence: {influence:.3}");
            }
            density += PARTICLE_MASS * influence;
        }
        if LOG_STUFF.load(Relaxed) {
            println!("density({:?}): {density}", pt.position);
        }
        density
    }

    pub fn smoothing_kernel(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            (self.smoothing_radius - distance) * (self.smoothing_radius - distance) / self.smoothing_scaling_factor
        }
    }

    pub fn smoothing_kernel_derivative(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            (distance - self.smoothing_radius) * self.smoothing_derivative_scaling_factor
        }
    }

    pub fn pressure(&self, density: f32) -> f32 {
        let density_error = density - self.target_density;
        density_error * PRESSURE_MULTIPLIER
    }

    pub fn resolve_collisions(&self, mut position: Vec2, mut velocity: Vec2) -> (Vec2, Vec2) {
        if position.x.abs() > self.half_bounds_size.x {
            position.x = self.half_bounds_size.x * position.x.signum();
            velocity.x *= -1.0 * COLLISION_DAMPING;
        }
        if position.y.abs() > self.half_bounds_size.y {
            position.y = self.half_bounds_size.y * position.y.signum();
            velocity.y *= -1.0 * COLLISION_DAMPING;
        }

        (position, velocity)
    }

    pub fn pressure_force(&self, pt: &Particle, particles: &Vec<Particle>) -> Vec2 {
        let mut gradient = Vec2::ZERO;
        let is_first_particle = pt.id == 0;

        for particle in particles {
            let offset = particle.position - pt.position;
            let distance = offset.length();
            if distance == 0.0 { continue; }
            // Unit vector in the direction of the particle.
            let direction = offset / distance;
            let slope = self.smoothing_kernel_derivative(distance);
            let pressure = self.pressure(particle.density);
            gradient += -pressure * direction * slope * PARTICLE_MASS / particle.density;
            if is_first_particle && LOG_STUFF.load(Relaxed) {
                println!("distance:{distance} direction:{direction} slope:{slope} pressure:{pressure} gradient:{gradient}");
            }
        }
        gradient
    }
}