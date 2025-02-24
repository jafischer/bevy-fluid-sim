use std::f32::consts::PI;

use bevy::prelude::*;
use rand::random;

use crate::Particle;
use crate::spatial_hash::{get_cell_2d, hash_cell_2d, key_from_hash, OFFSETS_2D};

#[derive(Component, Clone, Debug)]
pub struct Simulation {
    pub smoothing_radius: f32,
    pub smoothing_scaling_factor: f32,
    pub smoothing_derivative_scaling_factor: f32,
    pub num_particles: usize,
    pub particle_size: f32,
    pub scale: f32,
    pub half_bounds_size: Vec2,
    pub gravity: Vec2,
    pub target_density: f32,
    pub pressure_multiplier: f32,
    pub near_pressure_multiplier: f32,
    pub collision_damping: f32,

    // Particle information:
    pub positions: Vec<Vec2>,
    pub velocities: Vec<Vec2>,
    pub densities: Vec<f32>,

    debug: DebugParams,
}

#[derive(Clone, Debug)]
struct DebugParams {
    current_frame: u32,
    frames_to_show: u32,
    log_frame: u32,
    show_arrows: bool,
    use_gravity: bool,
}

impl Simulation {
    pub fn new(window_width: f32, window_height: f32) -> Simulation {
        let fluid_h = window_height * 0.67;
        let num_particles = 1000;
        let (grid_size, _, _) = Self::subdivide_into_squares(window_width, fluid_h, num_particles);

        // Because Sebastian's kernel math blows up with smoothing self.smoothing_radius values > 1, we don't want to use the
        // actual window coordinates. In Sebastian's video, at 5:40, he shows a smoothing self.smoothing_radius of 0.5
        // that is about 12 particles wide:
        // (https://youtu.be/rSKMYc1CQHE?si=3sibErk0e4CYC5wF&t=340)
        // So we want the grid size to be scaled down to 0.08333 (1/12).
        // grid_size * scale = 0.08333
        // scale = 0.08333 / grid_size
        let scale = 0.08333 / grid_size;
        let grid_size = grid_size * scale;
        let particle_size = grid_size * 0.5;
        let smoothing_radius = 0.2;

        let mut positions: Vec<Vec2> = Vec::with_capacity(num_particles);
        positions.resize_with(num_particles, Default::default);
        let mut velocities: Vec<Vec2> = Vec::with_capacity(num_particles);
        velocities.resize_with(num_particles, Default::default);
        let mut densities: Vec<f32> = Vec::with_capacity(num_particles);
        densities.resize_with(num_particles, Default::default);
        let mut spatial_offsets: Vec<u32> = Vec::with_capacity(num_particles);
        spatial_offsets.resize_with(num_particles, Default::default);
        let mut spatial_indices: Vec<[u32; 3]> = Vec::with_capacity(num_particles);
        spatial_indices.resize_with(num_particles, Default::default);
        let mut predicted_positions: Vec<Vec2> = Vec::with_capacity(num_particles);
        predicted_positions.resize_with(num_particles, Default::default);


        let sim = Simulation {
            smoothing_radius,
            smoothing_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
            smoothing_derivative_scaling_factor: PI * smoothing_radius.powf(4.0) / 6.0,
            num_particles,
            particle_size,
            scale,
            half_bounds_size: Vec2::new(window_width, window_height) * scale / 2.0 - particle_size / 2.0,
            gravity: Vec2::new(0.0, -10.0 * scale),
            target_density: 200.0, // TODO: calculate target_density based on window size & num_particles.
            pressure_multiplier: 500.0,
            near_pressure_multiplier: 50.0,
            collision_damping: 0.25,

            positions,
            velocities,
            densities,

            debug: DebugParams {
                current_frame: 0,
                frames_to_show: u32::MAX,
                log_frame: 0,
                show_arrows: false,
                use_gravity: true,
            },
        };

        println!("smoothing_radius: {}", sim.smoothing_radius);
        println!("smoothing_scaling_factor: {}", sim.smoothing_scaling_factor);
        println!("smoothing_derivative_scaling_factor: {}", sim.smoothing_derivative_scaling_factor);
        println!("num_particles: {}", sim.num_particles);
        println!("particle_size: {}", sim.particle_size);
        println!("scale: {}", sim.scale);
        println!("half_bounds_size: {}", sim.half_bounds_size);
        println!("gravity: {}", sim.gravity);
        println!("target_density: {}", sim.target_density);
        println!("pressure_multiplier: {}", sim.pressure_multiplier);
        println!("near_pressure_multiplier: {}", sim.near_pressure_multiplier);
        println!("collision_damping: {}", sim.collision_damping);

        sim
    }

    pub fn spawn_particles(
        &mut self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) {
        let color = Color::linear_rgb(0.0, 0.3, 1.0);

        self.place_particles();

        for i in 0..self.num_particles {
            let particle = Particle { id: i };

            commands.spawn((
                Mesh2d(meshes.add(Circle {
                    radius: self.particle_size / 2.0,
                })),
                MeshMaterial2d(materials.add(color)),
                Transform::from_translation(self.positions[i].extend(0.0)),
                particle,
            ));
        }
    }

    fn place_particles(&mut self) {
        for i in 0..self.num_particles {
            let x = -self.half_bounds_size.x + random::<f32>() * self.half_bounds_size.x * 2.0;
            let y = -self.half_bounds_size.y + random::<f32>() * self.half_bounds_size.y * 2.0;
            let velocity = Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5) * self.particle_size;
            self.positions[i] = Vec2::new(x, y);
            self.velocities[i] = velocity;
        }
    }
    
    pub fn calculate_densities(&mut self) {
        for i in 0..self.num_particles {
            self.densities[i] = self.density(i);
        }

        if self.debug.log_frame == self.debug.current_frame {
            // Log the density
            let highest_density = self.densities.clone().into_iter()
                .reduce(f32::max).unwrap();
            let average_density = self.densities.iter().sum::<f32>() / self.num_particles as f32;
            self.debug(format!("highest density: {highest_density}"));
            self.debug(format!("average density: {average_density}"));
        }
    }

    fn density(&self, id: usize) -> f32 {
        let position = self.positions[id];
        let mut density = 0.0;

        for (i, other_position) in self.positions.iter().enumerate() {
            if i == id {
                continue;
            }
            let distance = (other_position - position).length().max(0.000000001);
            let influence = self.smoothing_kernel(distance);
            density += influence;
        }
        density
    }
    
    pub fn calculate_pressures(&mut self, delta: f32) {
        self.velocities = (0..self.num_particles)
            .map(|i| self.calculate_pressure(i, delta))
            .collect();
    }

    fn calculate_pressure(&self, id: usize, delta: f32) -> Vec2 {
        let mut velocity = self.velocities[id];
        let pressure_force = self.pressure_force(id);
        if self.debug.use_gravity {
            velocity += (self.gravity + pressure_force) * delta;
        } else {
            velocity = pressure_force * delta;
        }
        // Poor man's viscosity:
        velocity = velocity.clamp_length_max(50.0 * self.particle_size * delta);
        
        velocity
    }

    pub fn apply_velocity(&mut self, id: usize) {
        self.positions[id] = self.positions[id] + self.velocities[id];
        self.resolve_collisions(id);
    }

    pub fn reset(&mut self) {
        self.place_particles();
    }

    pub fn on_resize(&mut self, window_width: f32, window_height: f32) {
        self.half_bounds_size = Vec2::new(window_width, window_height) * self.scale / 2.0 - self.particle_size / 2.0;
    }

    pub fn end_frame(&mut self) {
        self.debug.current_frame += 1;
        if self.debug.frames_to_show > 0 {
            self.debug.frames_to_show -= 1;
        }
    }

    pub fn frames_to_advance(&self) -> u32 {
        self.debug.frames_to_show
    }

    pub fn set_frames_to_show(&mut self, val: u32) {
        self.debug.frames_to_show = val;
    }

    pub fn debug(&self, message: String) {
        if self.debug.log_frame == self.debug.current_frame {
            println!("{message}");
        }
    }

    pub fn show_arrows(&self) -> bool {
        self.debug.show_arrows
    }

    pub fn toggle_arrows(&mut self) {
        self.debug.show_arrows = !self.debug.show_arrows;
    }

    pub fn toggle_gravity(&mut self) {
        self.debug.use_gravity = !self.debug.use_gravity;
    }

    pub fn log_next_frame(&mut self) {
        self.debug.log_frame = self.debug.current_frame + 1;
    }

    pub fn inc_smoothing_radius(&mut self) {
        self.smoothing_radius += 0.1;
        self.smoothing_scaling_factor = 6.0 / (PI * self.smoothing_radius.powf(4.0));
        self.smoothing_derivative_scaling_factor = PI * self.smoothing_radius.powf(4.0) / 6.0;
        println!("smoothing_radius: {}", self.smoothing_radius);
        self.log_next_frame();
    }

    pub fn dec_smoothing_radius(&mut self) {
        if self.smoothing_radius > 0.1 {
            self.smoothing_radius -= 0.1;
            self.smoothing_scaling_factor = 6.0 / (PI * self.smoothing_radius.powf(4.0));
            self.smoothing_derivative_scaling_factor = PI * self.smoothing_radius.powf(4.0) / 6.0;
            println!("smoothing_radius: {}", self.smoothing_radius);
            self.log_next_frame();
        }
    }

    fn smoothing_kernel(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            (self.smoothing_radius - distance) * (self.smoothing_radius - distance) * self.smoothing_scaling_factor
        }
    }

    fn smoothing_kernel_derivative(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            (distance - self.smoothing_radius) * self.smoothing_derivative_scaling_factor
        }
    }

    fn shared_pressure(&self, density1: f32, density2: f32) -> f32 {
        let density_error1 = density1 - self.target_density;
        let density_error2 = density2 - self.target_density;
        (density_error1 + density_error2) * self.pressure_multiplier / 2.0
    }

    fn resolve_collisions(&mut self, id: usize) {
        if self.positions[id].x.abs() > self.half_bounds_size.x {
            self.positions[id].x = self.half_bounds_size.x * self.positions[id].x.signum();
            self.velocities[id].x *= -1.0 * self.collision_damping;
        }
        if self.positions[id].y.abs() > self.half_bounds_size.y {
            self.positions[id].y = self.half_bounds_size.y * self.positions[id].y.signum();
            self.velocities[id].y *= -1.0 * self.collision_damping;
        }
    }

    fn pressure_force(&self, id: usize) -> Vec2 {
        let mut gradient = Vec2::default();
        let position = self.positions[id];
        let density = self.densities[id];

        for i in 0..self.num_particles {
            if i == id {
                continue;
            }
            let offset = self.positions[i] - position;
            let distance = offset.length().max(0.000000001);
            if distance >= self.smoothing_radius {
                continue;
            }
            // Unit vector in the direction of the particle.
            let direction = offset / distance;
            let slope = self.smoothing_kernel_derivative(distance);
            let pressure = self.shared_pressure(density, self.densities[i]);
            gradient += direction * slope * pressure / self.densities[i];
        }
        gradient // / pt.density
    }

    /// Divides a rectangular region into (roughly) n squares.
    fn subdivide_into_squares(w: f32, h: f32, n: usize) -> (f32, usize, usize) {
        // Step 1: Calculate the target area of each square
        let target_area = (w * h) / n as f32;

        // Step 2: Calculate the side length of each square
        let side_length = target_area.sqrt();

        // Step 3: Calculate the number of columns and rows
        let columns = w / side_length;
        let rows = n as f32 / columns;

        // Step 4: Adjust the final side length to fit evenly
        let side_length = f32::min(w / columns, h / rows);

        (side_length, columns as usize, rows as usize)
    }
}
