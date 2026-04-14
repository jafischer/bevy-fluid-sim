use std::f32::consts::PI;

use bevy::prelude::*;
use rand::random;
use rayon::prelude::*;

use crate::args::Args;
use crate::sim_struct::{DebugParams, Simulation};
use crate::Particle;

const OFFSETS_2D: [(i32, i32); 9] = [
    (-1, 1),
    (0, 1),
    (1, 1),
    (-1, 0),
    (0, 0),
    (1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

impl Simulation {
    pub fn new(window_width: f32, window_height: f32, args: &Args) -> Simulation {
        let window_area = window_width * window_height;
        // Pick a particle size (in pixels) relative to the window size.
        let particle_size = (window_area * 0.5 / args.num_particles as f32).sqrt();

        // Preallocate the vectors.
        let positions = vec![Vec2::default(); args.num_particles];
        let predicted_positions = vec![Vec2::default(); args.num_particles];
        let velocities = vec![Vec2::default(); args.num_particles];
        let densities = vec![0f32; args.num_particles];

        let mut sim = Simulation {
            smoothing_radius: 0.0,
            smoothing_scaling_factor: 0.0,
            smoothing_derivative_scaling_factor: 0.0,
            viscosity_scaling_factor: 0.0,
            num_particles: args.num_particles,
            particle_size,
            sprite_size: args.sprite_size,
            half_bounds_size: Vec2::new(window_width, window_height) / 2.0 - particle_size / 2.0,
            gravity: Vec2::new(0.0, args.gravity * particle_size),
            target_density: 0.0,
            pressure_multiplier: args.pressure_multiplier as f32 * particle_size,
            collision_damping: args.collision_damping,
            speed: args.speed,

            viscosity_strength: args.viscosity_strength,
            interaction_input_strength: 0.0,
            interaction_input_radius: args.interaction_input_radius as f32 * particle_size,
            interaction_input_point: Vec2::ZERO,

            positions,
            predicted_positions,
            velocities,
            densities,
            region_rows: 0,
            region_cols: 0,
            regions: vec![],
            min_velocity: f32::MAX,
            max_velocity: 0.0,
            min_density: f32::MAX,
            max_density: 0.0,

            debug: DebugParams {
                current_frame: 0,
                frames_to_show: u32::MAX,
                log_frame: u32::MAX,
                show_fps: false,
                show_smoothing_radius: false,
                show_region_grid: false,
                use_heatmap: true,
                show_arrows: false,
                use_predicted_positions: false,
            },
        };

        sim.set_smoothing_radius(args.smoothing_radius);

        sim
    }

    pub fn set_smoothing_radius(&mut self, smoothing_radius: f32) {
        let smoothing_radius = smoothing_radius * self.particle_size;

        self.smoothing_radius = smoothing_radius;
        // The scaling factors are the volume of the corresponding kernel functions over the smoothing radius.
        self.smoothing_scaling_factor = 10.0 / (PI * smoothing_radius.powf(5.0));
        self.smoothing_derivative_scaling_factor = 30.0 / (PI * smoothing_radius.powf(5.0));
        self.viscosity_scaling_factor = 6.0 / (PI * smoothing_radius.powf(4.0));
    }

    pub fn spawn_particles(&mut self, commands: &mut Commands) {
        let color = Color::linear_rgb(0.3, 0.5, 1.0);

        self.place_particles();

        for i in 0..self.num_particles {
            let particle = Particle { id: i, watched: false };

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(self.particle_size * self.sprite_size)),
                    ..Default::default()
                },
                particle,
            ));
        }
    }

    pub fn place_particles(&mut self) {
        let (grid_size, cols, rows) = self.subdivide_into_squares();

        let pos_start = Vec2 {
            x: -self.half_bounds_size.x,
            y: -self.half_bounds_size.y * 0.8,
        };

        for i in 0..self.num_particles {
            let row = i / cols;
            let col = i % cols;

            let x = pos_start.x + col as f32 * grid_size * 0.8;
            let y = pos_start.y + row as f32 * grid_size * 0.9;
            self.positions[i] = Vec2::new(x, y);
            self.predicted_positions[i] = self.positions[i];
            self.velocities[i] = Vec2::ZERO;
        }

        self.update_regions();

        // Set the target density based on the current density of the center particle.
        if self.target_density == 0.0 {
            self.target_density = self.calculate_density((rows / 2) * cols + (cols / 2)) * 0.8;
            println!("Target density: {}", self.target_density);
        }
    }

    pub fn update_particles(&mut self, delta: f32) {
        self.update_regions();

        self.predicted_positions = (0..self.num_particles)
            .into_par_iter()
            .map(|i| self.positions[i] + self.velocities[i] * delta * self.speed)
            .collect();

        self.calculate_densities();

        if self.frames_to_advance() > 0 {
            self.calculate_pressures(delta);
            self.apply_velocities(delta);
            self.apply_viscosity();

            self.min_velocity = f32::MAX;
            self.max_velocity = 0.0;
            self.min_density = f32::MAX;
            self.max_density = 0.0;

            for i in 0..self.num_particles {
                self.min_density = self.min_density.min(self.densities[i]);
                self.max_density = self.max_density.max(self.densities[i]);
                self.min_velocity = self.min_velocity.min(self.velocities[i].length());
                self.max_velocity = self.max_velocity.max(self.velocities[i].length());
            }
        }
    }

    pub fn on_resize(&mut self, window_width: f32, window_height: f32) {
        self.half_bounds_size = Vec2::new(window_width, window_height) / 2.0 - self.particle_size / 2.0;
    }

    pub fn end_frame(&mut self) {
        if self.debug.log_frame == self.debug.current_frame {
            println!("{self:?}");
            println!();
            let lowest_density = self.densities.iter().cloned().reduce(f32::min).unwrap();
            let highest_density = self.densities.iter().cloned().reduce(f32::max).unwrap();
            let average_density = self.densities.iter().cloned().sum::<f32>() / self.num_particles as f32;

            let lowest_velocity = self.velocities.iter().map(|v| v.length()).reduce(f32::min).unwrap();
            let highest_velocity = self.velocities.iter().map(|v| v.length()).reduce(f32::max).unwrap();
            let average_velocity = self.velocities.iter().map(|v| v.length()).sum::<f32>() / self.num_particles as f32;
            println!("density:  min:     {}", self.min_density);
            println!("          lowest:  {lowest_density}");
            println!("          highest: {highest_density}");
            println!("          max:     {}", self.max_density);
            println!("          avg:     {average_density}");
            println!("velocity: min:     {}", self.min_velocity);
            println!("          lowest:  {lowest_velocity}");
            println!("          highest: {highest_velocity}");
            println!("          max:     {}", self.max_velocity);
            println!("          avg:     {average_velocity}");
        }

        self.debug.current_frame += 1;
        if self.debug.frames_to_show > 0 {
            self.debug.frames_to_show -= 1;
        }
    }

    fn subdivide_into_squares(&self) -> (f32, usize, usize) {
        let width = self.half_bounds_size.x * 2.0;
        let height = self.half_bounds_size.y * 2.0;
        let target_area = (width * height) / self.num_particles as f32;
        let side_length = target_area.sqrt();
        let columns = (width / side_length) as usize;
        let rows = (self.num_particles as f32 / columns as f32) as usize;

        // Adjust the final side length to fit evenly
        let side_length = f32::min(width / columns as f32, height / rows as f32);

        (side_length, columns, rows)
    }

    /// This is my simplistic alternative to the funky "spatial hash" code.
    /// I just divide the space up into regions the size of the smoothing hash, and
    /// keep track of the particles in each region. Wasteful of memory, but it's simple and
    /// it works.
    fn update_regions(&mut self) {
        let width = self.half_bounds_size.x * 2.0;
        let height = self.half_bounds_size.y * 2.0;
        let cols = (width / self.smoothing_radius) as usize + 1;
        let rows = (height / self.smoothing_radius) as usize + 1;
        let num_regions = rows * cols;

        // If window size or smoothing radius has changed, need to resize the regions vector.
        if self.region_rows != rows || self.region_cols != cols {
            self.region_rows = rows;
            self.region_cols = cols;
            self.regions.clear();
            for row in 0..rows {
                self.regions.push(vec![]);
                for _ in 0..cols {
                    let region = Vec::with_capacity(self.num_particles / num_regions * 4);
                    self.regions[row].push(region);
                }
            }
        } else {
            for row in 0..rows {
                for col in 0..cols {
                    self.regions[row][col].clear();
                }
            }
        }

        let left = -self.half_bounds_size.x;
        let bottom = -self.half_bounds_size.y;
        for i in 0..self.num_particles {
            let col = ((self.positions[i].x - left) / self.smoothing_radius) as usize;
            let row = ((self.positions[i].y - bottom) / self.smoothing_radius) as usize;
            // While the window is being resized, some particles can be temporarily
            // outside the window.
            let col = col.clamp(0, cols - 1);
            let row = row.clamp(0, rows - 1);
            self.regions[row][col].push(i);
        }
    }

    fn calculate_densities(&mut self) {
        self.densities = (0..self.num_particles)
            .into_par_iter()
            .map(|i| self.calculate_density(i))
            .collect();
    }

    fn calculate_density(&self, particle_id: usize) -> f32 {
        let position = if self.debug.use_predicted_positions {
            self.predicted_positions[particle_id]
        } else {
            self.positions[particle_id]
        };
        let mut density = 0.0;

        for neighbor_id in self.neighbor_particles(particle_id) {
            let neighbor_pos = if self.debug.use_predicted_positions {
                self.predicted_positions[neighbor_id]
            } else {
                self.positions[neighbor_id]
            };
            let distance = (neighbor_pos - position).length().max(0.000000001);
            let influence = self.smoothing_kernel(distance);
            density += influence;
        }

        density
    }

    fn calculate_pressures(&mut self, delta: f32) {
        self.velocities = (0..self.num_particles)
            .into_par_iter()
            .map(|i| self.calculate_pressure(i, delta))
            .collect();
    }

    fn calculate_pressure(&self, particle_id: usize, delta: f32) -> Vec2 {
        self.velocities[particle_id]
            + self.pressure_force(particle_id) * delta
            + self.gravity_force(particle_id) * delta
    }

    fn neighbor_particles(&self, particle_id: usize) -> impl Iterator<Item = usize> + '_ {
        let particle_row = ((self.positions[particle_id].y - -self.half_bounds_size.y) / self.smoothing_radius) as i32;
        let particle_col = ((self.positions[particle_id].x - -self.half_bounds_size.x) / self.smoothing_radius) as i32;

        OFFSETS_2D.iter().flat_map(move |offset| {
            let region_row = particle_row + offset.0;
            let region_col = particle_col + offset.1;
            let in_bounds = region_row >= 0
                && (region_row as usize) < self.region_rows
                && region_col >= 0
                && (region_col as usize) < self.region_cols;
            if in_bounds { self.regions[region_row as usize][region_col as usize].as_slice() } else { &[] }
                .iter()
                .copied()
                .filter(move |&neighbor_id| neighbor_id != particle_id)
        })
    }

    fn apply_velocities(&mut self, delta: f32) {
        (self.positions, self.velocities) = (0..self.num_particles)
            .into_par_iter()
            .map(|particle_id| self.apply_velocity(particle_id, delta))
            .unzip();
    }

    fn apply_viscosity(&mut self) {
        self.velocities = (0..self.num_particles)
            .into_par_iter()
            .map(|particle_id| self.apply_viscosity_to_particle(particle_id))
            .collect();
    }

    fn apply_velocity(&self, particle_id: usize, delta: f32) -> (Vec2, Vec2) {
        let position = self.positions[particle_id] + self.velocities[particle_id] * delta * self.speed;
        self.resolve_collisions(position, self.velocities[particle_id])
    }

    fn smoothing_kernel(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0.0
        } else {
            let value = self.smoothing_radius - distance;
            value * value * value * self.smoothing_scaling_factor
        }
    }

    fn smoothing_kernel_derivative(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0.0
        } else {
            2.0 * (distance - self.smoothing_radius)
                * (distance - self.smoothing_radius)
                * self.smoothing_derivative_scaling_factor
        }
    }

    fn viscosity_kernel(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0.0
        } else {
            (self.smoothing_radius - distance) * (self.smoothing_radius - distance) * self.viscosity_scaling_factor
        }
    }

    fn shared_pressure(&self, density1: f32, density2: f32) -> f32 {
        let density_error1 = density1 - self.target_density;
        let density_error2 = density2 - self.target_density;
        (density_error1 + density_error2) * self.pressure_multiplier / 2.0
    }

    fn resolve_collisions(&self, mut position: Vec2, mut velocity: Vec2) -> (Vec2, Vec2) {
        if position.x.abs() > self.half_bounds_size.x {
            position.x = self.half_bounds_size.x * position.x.signum();
            velocity.x = (velocity.x * self.collision_damping).abs() * -position.x.signum();
        }
        if position.y.abs() > self.half_bounds_size.y {
            position.y = self.half_bounds_size.y * position.y.signum();
            velocity.y = (velocity.y * self.collision_damping).abs() * -position.y.signum();
        }

        (position, velocity)
    }

    fn pressure_force(&self, particle_id: usize) -> Vec2 {
        let mut pressure_force = Vec2::default();
        let position = self.positions[particle_id];
        let density = self.densities[particle_id];

        for neighbor_id in self.neighbor_particles(particle_id) {
            let offset = self.positions[neighbor_id] - position;
            let distance = offset.length();
            if distance < self.smoothing_radius {
                if distance > 0.0 {
                    let direction = -(offset / distance);
                    let slope = self.smoothing_kernel_derivative(distance);
                    let pressure = self.shared_pressure(density, self.densities[neighbor_id]);
                    pressure_force += pressure * direction * slope / self.densities[neighbor_id];
                } else {
                    // Move toward the center, plus a random vector.
                    pressure_force += (Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5)
                        + (Vec2::ZERO - position))
                        * self.particle_size;
                }
            }
        }

        pressure_force
    }

    fn gravity_force(&self, particle_id: usize) -> Vec2 {
        let pos = self.positions[particle_id];
        let velocity = self.velocities[particle_id];

        // Mouse buttons generate pseudo gravity/repulsion at mouse location.
        if self.interaction_input_strength != 0.0 {
            let input_point_offset = self.interaction_input_point - pos;
            let sqr_distance = input_point_offset.dot(input_point_offset);
            if sqr_distance < self.interaction_input_radius * self.interaction_input_radius {
                let distance = sqr_distance.sqrt();
                let edge_t = distance / self.interaction_input_radius;
                let centre_t = 1.0 - edge_t;
                let dir_to_centre = input_point_offset / distance;

                let gravity_weight = 1.0 - (centre_t * (self.interaction_input_strength / 10.0).clamp(0.0, 1.0));
                let mut accel =
                    -self.gravity * gravity_weight + dir_to_centre * centre_t * self.interaction_input_strength;
                accel -= velocity * centre_t;
                return accel;
            }
        }

        -self.gravity
    }

    fn apply_viscosity_to_particle(&self, particle_id: usize) -> Vec2 {
        let velocity = self.velocities[particle_id];
        let position = self.positions[particle_id];
        let mut viscosity = Vec2::default();

        for neighbor_id in self.neighbor_particles(particle_id) {
            let offset = self.positions[neighbor_id] - position;
            let distance = offset.length().max(0.00000001);
            if distance < self.smoothing_radius {
                let influence = self.viscosity_kernel(distance);
                viscosity += (self.velocities[neighbor_id] - velocity) * influence;
            }
        }

        velocity + viscosity * self.viscosity_strength
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec2;

    use super::*;

    /// Place particles in an evenly-spaced grid and verify that the
    /// density of the center particle is approximately the same across
    /// different smoothing radii.
    #[test]
    fn center_density_is_independent_of_smoothing_radius() {
        let rows = 40;
        let cols = 40;
        let num_particles = rows * cols;

        for window_scale in [0.01, 0.1, 1.0, 2.0, 10.0] {
            let win_width = rows as f32 * window_scale;
            let win_height = cols as f32 * window_scale;
            let center_particle = (rows / 2) * cols + (cols / 2);
            let mut densities = Vec::new();
            let mut pressures = Vec::new();

            let mut sim = Simulation::new(
                win_width,
                win_height,
                &Args {
                    win: "".to_string(),
                    num_particles,
                    smoothing_radius: 0.0,
                    gravity: 0.0,
                    speed: 0.0,
                    pressure_multiplier: 0,
                    viscosity_strength: 0.0,
                    collision_damping: 0.0,
                    interaction_input_radius: 0,
                    interaction_input_strength: 0.0,
                    sprite_size: 0.0,
                },
            );
            let spacing = sim.particle_size * 1.5;

            println!("\nwindow scale: {window_scale}, particle size: {}", sim.particle_size);

            // Place particles in an evenly-spaced grid.
            for row in 0..rows {
                for col in 0..cols {
                    let particle_id = row * cols + col;
                    let x = col as f32 * spacing;
                    let y = row as f32 * spacing;
                    sim.positions[particle_id] = Vec2::new(x, y);
                    sim.predicted_positions[particle_id] = sim.positions[particle_id];
                }
            }

            for smoothing_radius in [10.0, 15.0, 20.0, 25.0, 30.0] {
                sim.set_smoothing_radius(smoothing_radius);
                sim.update_regions();

                println!("smoothing_radius: {smoothing_radius:.4} -> {}", sim.smoothing_radius);

                let density = sim.calculate_density(center_particle);
                assert!(density > 0.0);
                densities.push(density);
                println!("    density={density:.4}");

                sim.calculate_densities();
                let pressure = sim.calculate_pressure(center_particle, 1.0 / 120.0) / sim.particle_size;
                assert_ne!(Vec2::ZERO, pressure);
                pressures.push(pressure);
                println!("    pressure={pressure:.4}");
            }

            let mean = densities.iter().sum::<f32>() / densities.len() as f32;
            let mut max_diff = 0f32;
            for &d in densities.iter() {
                let relative_diff = (d - mean).abs() / mean;
                println!("    relative_diff={:.1}%", 100.0 * relative_diff);
                max_diff = max_diff.max(relative_diff);
            }

            // All densities should be approximately equal (within 5% of the mean).
            assert!(max_diff < 0.05);
        }
    }
}
