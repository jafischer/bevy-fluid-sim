use std::f32::consts::PI;

use bevy::prelude::*;
use rand::random;
use rayon::prelude::*;

use crate::args::ARGS;
use crate::sim_struct::{DebugParams, Simulation};
use crate::spatial_hash::OFFSETS_2D;
use crate::Particle;

impl Simulation {
    pub fn new(window_width: f32, window_height: f32) -> Simulation {
        let num_particles = ARGS.num as usize;
        let window_area = window_width * window_height;
        // Pick a particle size such that the "fluid" will fill roughly 2/3 the window.
        let particle_size = (window_area * 0.67 / num_particles as f32).sqrt();

        // The kernel math blows up with smoothing radius values > 1, we don't want to use the
        // actual window coordinates. So I'm just hacking up a scale factor for now.
        // In Sebastian's video, at 5:40, he shows a smoothing radius of 0.5
        // that is about 12 particles wide:
        // (https://youtu.be/rSKMYc1CQHE?si=3sibErk0e4CYC5wF&t=340)
        // So we'll just scale down the grid size to 0.08333 (1/12).
        // grid_size * scale = 0.08333
        // --> scale = 0.08333 / grid_size, see, I can still do grade school math.
        let scale = 0.08333 / particle_size;
        let particle_size = particle_size * scale;
        let smoothing_radius = ARGS.smoothing_radius;

        let mut positions = vec![];
        positions.resize_with(num_particles, Default::default);
        let mut velocities = vec![];
        velocities.resize_with(num_particles, Default::default);
        let mut densities = vec![];
        densities.resize_with(num_particles, Default::default);
        let mut spatial_offsets = vec![];
        spatial_offsets.resize_with(num_particles, Default::default);
        let mut spatial_indices = vec![];
        spatial_indices.resize_with(num_particles, Default::default);
        let mut spatial_keys = vec![];
        spatial_keys.resize_with(num_particles, Default::default);
        let mut predicted_positions = vec![];
        predicted_positions.resize_with(num_particles, Default::default);

        let sim = Simulation {
            smoothing_radius,
            smoothing_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
            smoothing_derivative_scaling_factor: PI * smoothing_radius.powf(4.0) / 6.0,
            num_particles,
            particle_size,
            scale,
            half_bounds_size: Vec2::new(window_width, window_height) * scale / 2.0 - particle_size / 2.0,
            gravity: Vec2::new(0.0, -ARGS.gravity),
            target_density: 1.5 / scale,
            pressure_multiplier: ARGS.pressure_multiplier as f32,
            near_pressure_multiplier: 100.0, // TODO: ARGS.near_pressure_multiplier as f32,
            viscosity_strength: 0.0,
            speed_limit: ARGS.speed_limit,
            collision_damping: ARGS.collision_damping,

            interaction_input_strength: 0.0,
            interaction_input_radius: ARGS.interaction_input_radius as f32 * particle_size,
            interaction_input_point: Vec2::ZERO,

            positions,
            velocities,
            densities,
            region_rows: 0,
            region_cols: 0,
            regions: vec![],

            // I've copied some stuff from Sebastian's Fluid-Sim compute shader code, but haven't
            // integrated it yet.
            prediction_factor: 1.0 / 120.0,
            predicted_positions,
            spatial_offsets,
            spatial_keys,
            spatial_indices,
            poly6_scaling_factor: 4.0 / (PI * smoothing_radius.powf(8.0)),
            spiky_pow3_scaling_factor: 10.0 / (PI * smoothing_radius.powf(5.0)),
            spiky_pow2_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
            spiky_pow3_derivative_scaling_factor: 30.0 / (smoothing_radius.powf(5.0) * PI),
            spiky_pow2_derivative_scaling_factor: 12.0 / (smoothing_radius.powf(4.0) * PI),

            debug: DebugParams {
                use_sfs: false,
                current_frame: 0,
                frames_to_show: u32::MAX,
                log_frame: 0,
                show_fps: false,
                show_smoothing_radius: false,
                show_region_grid: false,
                use_inertia: true,
                use_viscosity: true,
                use_heatmap: true,
            },
        };

        println!("{sim:?}");

        sim
    }

    pub fn spawn_particles(&mut self, commands: &mut Commands) {
        let color = Color::linear_rgb(0.3, 0.5, 1.0);

        self.place_particles();

        for i in 0..self.num_particles {
            let particle = Particle { id: i, watched: false };

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(self.particle_size * ARGS.sprite_size)),
                    ..Default::default()
                },
                particle,
            ));
        }
    }

    pub fn place_particles(&mut self) {
        let bounds = self.half_bounds_size * 0.8;
        for i in 0..self.num_particles {
            let x = -bounds.x + random::<f32>() * bounds.x * 2.0;
            let y = -bounds.y + random::<f32>() * bounds.y * 2.0;
            self.positions[i] = Vec2::new(x, y);
            self.predicted_positions[i] = self.positions[i];
            self.velocities[i] = Vec2::ZERO;
        }
    }

    pub fn update_particles(&mut self, delta: f32) {
        if self.debug.use_sfs {
            self.sfs_update_spatial_hash();
        } else {
            self.update_regions();
        }

        self.calculate_densities();
        if self.frames_to_advance() > 0 {
            // TODO:
            // if self.debug.use_sfs {
            //     self.sfs_
            // } else {
            self.calculate_pressures(delta);
            self.apply_velocities();
            // }
        }
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
        if self.debug.use_sfs {
            self.densities = (0..self.num_particles)
                .into_par_iter()
                .map(|i| self.sfs_calculate_density(&self.positions[i]))
                .collect();
        } else {
            self.densities = (0..self.num_particles)
                .into_par_iter()
                .map(|i| self.calculate_density(i))
                .collect();
        }

        if self.debug.log_frame == self.debug.current_frame {
            let lowest_density = self
                .densities
                .iter()
                .map(|(density, _)| *density)
                .reduce(f32::min)
                .unwrap();
            let highest_density = self
                .densities
                .clone()
                .iter()
                .map(|(density, _)| *density)
                .reduce(f32::max)
                .unwrap();
            let average_density =
                self.densities.iter().map(|(density, _)| *density).sum::<f32>() / self.num_particles as f32;
            println!("lowest density: {lowest_density}");
            println!("highest density: {highest_density}");
            println!("average density: {average_density}");
        }
    }

    fn calculate_density(&self, id: usize) -> (f32, f32) {
        let position = self.positions[id];
        let mut density = 0.0;

        let bottom = -self.half_bounds_size.y;
        let left = -self.half_bounds_size.x;
        let particle_row = ((self.positions[id].y - bottom) / self.smoothing_radius) as usize;
        let particle_col = ((self.positions[id].x - left) / self.smoothing_radius) as usize;

        for offset in OFFSETS_2D {
            let region_row = particle_row as i32 + offset.0;
            let region_col = particle_col as i32 + offset.1;
            if region_row >= 0 && region_col >= 0 {
                let region_row = region_row as usize;
                let region_col = region_col as usize;
                if region_row < self.region_rows && region_col < self.region_cols {
                    let row = &self.regions[region_row];
                    for i in &row[region_col] {
                        if *i == id {
                            continue;
                        }
                        // let neighbor_pos = self.predicted_positions[*i];
                        let neighbor_pos = self.positions[*i];
                        let distance = (neighbor_pos - position).length().max(0.000000001);
                        let influence = self.smoothing_kernel(distance);
                        // let influence = self.density_kernel(distance);
                        density += influence;
                    }
                }
            }
        }
        (density, density)
    }

    pub fn calculate_pressures(&mut self, delta: f32) {
        self.velocities = (0..self.num_particles)
            .into_par_iter()
            .map(|i| self.calculate_pressure(i, delta))
            .collect();

        self.predicted_positions = (0..self.num_particles)
            .into_par_iter()
            .map(|i| self.positions[i] + self.velocities[i] * self.prediction_factor)
            .collect();
    }

    fn calculate_pressure(&self, id: usize, delta: f32) -> Vec2 {
        let mut velocity = self.velocities[id];
        let pressure_force = self.pressure_force(id) * delta;
        // let pressure_force = self.sfs_calculate_pressure_force(id) * delta;
        let gravity_force = self.external_forces(id) * delta;

        if self.debug.use_inertia {
            // Poor man's viscosity:
            if self.debug.use_viscosity {
                velocity = (velocity + pressure_force).clamp_length_max(self.speed_limit * self.particle_size * delta);
            } else {
                velocity += pressure_force;
            }
            velocity += gravity_force;
        } else {
            velocity = pressure_force + gravity_force;
        }

        velocity
    }

    pub fn apply_velocities(&mut self) {
        for i in 0..self.num_particles {
            self.apply_velocity(i);
        }
    }

    fn apply_velocity(&mut self, id: usize) {
        self.positions[id] = self.positions[id] + self.velocities[id];
        self.resolve_collisions(id);
    }

    pub fn on_resize(&mut self, window_width: f32, window_height: f32) {
        self.half_bounds_size = Vec2::new(window_width, window_height) * self.scale / 2.0 - self.particle_size / 2.0;
    }

    pub fn end_frame(&mut self) {
        if self.debug.log_frame == self.debug.current_frame {
            println!("{self:?}");
        }
        self.debug.current_frame += 1;
        if self.debug.frames_to_show > 0 {
            self.debug.frames_to_show -= 1;
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
        let density = self.densities[id].0;

        let bottom = -self.half_bounds_size.y;
        let left = -self.half_bounds_size.x;
        let particle_row = ((self.positions[id].y - bottom) / self.smoothing_radius) as usize;
        let particle_col = ((self.positions[id].x - left) / self.smoothing_radius) as usize;

        for offset in OFFSETS_2D {
            let region_row = particle_row as i32 + offset.0;
            let region_col = particle_col as i32 + offset.1;
            if region_row >= 0 && region_col >= 0 {
                let region_row = region_row as usize;
                let region_col = region_col as usize;
                if region_row < self.region_rows && region_col < self.region_cols {
                    let row = &self.regions[region_row];
                    for i in &row[region_col] {
                        if *i == id {
                            continue;
                        }
                        let offset = self.positions[*i] - position;
                        let distance = offset.length();
                        if distance >= self.smoothing_radius {
                            continue;
                        }
                        if distance == 0.0 {
                            // Move toward the center, plus a random vector.
                            gradient += (Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5)
                                + (Vec2::ZERO - position))
                                * self.particle_size;

                            continue;
                        }

                        // Unit vector in the direction of the particle.
                        let direction = offset / distance;
                        let slope = self.smoothing_kernel_derivative(distance);
                        let pressure = self.shared_pressure(density, self.densities[*i].0);
                        gradient += direction * slope * pressure / self.densities[*i].0;
                    }
                }
            }
        }

        gradient
    }

    fn external_forces(&self, id: usize) -> Vec2 {
        let pos = self.positions[id];
        let velocity = self.velocities[id];
        // Input interactions modify gravity
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
                    self.gravity * gravity_weight + dir_to_centre * centre_t * self.interaction_input_strength;
                accel -= velocity * centre_t;
                return accel * self.scale;
            }
        }

        self.gravity * self.scale
    }
}
