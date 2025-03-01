use std::f32::consts::PI;
use std::fmt::{Debug, Formatter};
use bevy::prelude::*;
use rand::random;
use rayon::prelude::*;

use crate::args::ARGS;
use crate::spatial_hash::{get_cell_2d, hash_cell_2d, key_from_hash, OFFSETS_2D};
use crate::Particle;

#[derive(Component)]
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
    pub speed_limit: f32,
    pub collision_damping: f32,

    // For attraction/repulsion effect when mouse is clicked:
    pub interaction_input_strength: f32,
    pub interaction_input_radius: f32,
    pub interaction_input_point: Vec2,

    // Particle information:
    pub positions: Vec<Vec2>,
    pub velocities: Vec<Vec2>,
    pub densities: Vec<(f32, f32)>,
    pub region_rows: usize,
    pub region_cols: usize,
    pub regions: Vec<Vec<Vec<usize>>>,

    // Fluid-Sim fields:
    pub predicted_positions: Vec<Vec2>,
    pub spatial_offsets: Vec<u32>,
    pub spatial_indices: Vec<[u32; 3]>,
    pub poly6_scaling_factor: f32,
    pub spiky_pow3_scaling_factor: f32,
    pub spiky_pow2_scaling_factor: f32,
    pub spiky_pow3_derivative_scaling_factor: f32,
    pub spiky_pow2_derivative_scaling_factor: f32,

    pub debug: DebugParams,
}

impl Debug for Simulation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Simulation:")?;
        writeln!(f, "    smoothing_radius: {}", self.smoothing_radius)?;
        writeln!(f, "    num_particles: {}", self.num_particles)?;
        writeln!(f, "    scale: {}", self.scale)?;
        writeln!(f, "    particle_size: {}", self.particle_size)?;
        writeln!(f, "    gravity: {}", self.gravity)?;
        writeln!(f, "    target_density: {}", self.target_density)?;
        writeln!(f, "    pressure_multiplier: {}", self.pressure_multiplier)?;
        writeln!(f, "    near_pressure_multiplier: {}", self.near_pressure_multiplier)?;
        writeln!(f, "    collision_damping: {}", self.collision_damping)
    }
}

pub struct DebugParams {
    pub current_frame: u32,
    pub frames_to_show: u32,
    pub log_frame: u32,
    pub show_arrows: bool,
    pub show_smoothing_radius: bool,
    pub show_region_grid: bool,
    pub use_inertia: bool,
    pub use_viscosity: bool,
    pub use_heatmap: bool,
}

impl Simulation {
    pub fn new(window_width: f32, window_height: f32) -> Simulation {
        let num_particles = ARGS.num as usize;
        // Pick a particle size such that the "fluid" will fill roughly 2/3 the window.
        let window_area = window_width * window_height;
        let particle_size = (window_area * 0.67 / num_particles as f32).sqrt();

        // Because Sebastian's kernel math blows up with smoothing radius values > 1, we don't want to use the
        // actual window coordinates. In Sebastian's video, at 5:40, he shows a smoothing self.smoothing_radius of 0.5
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
            near_pressure_multiplier: ARGS.near_pressure_multiplier as f32,
            speed_limit: 50.0,
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

            predicted_positions,
            spatial_offsets,
            spatial_indices,
            poly6_scaling_factor: 4.0 / (PI * smoothing_radius.powf(8.0)),
            spiky_pow3_scaling_factor: 10.0 / (PI * smoothing_radius.powf(5.0)),
            spiky_pow2_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
            spiky_pow3_derivative_scaling_factor: 30.0 / (smoothing_radius.powf(5.0) * PI),
            spiky_pow2_derivative_scaling_factor: 12.0 / (smoothing_radius.powf(4.0) * PI),

            debug: DebugParams {
                current_frame: 0,
                frames_to_show: u32::MAX,
                log_frame: 0,
                show_arrows: false,
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

    pub fn spawn_particles(
        &mut self,
        commands: &mut Commands,
    ) {
        let color = Color::linear_rgb(0.3, 0.5, 1.0);

        self.place_particles();

        for i in 0..self.num_particles {
            let particle = Particle { id: i, watched: false };

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(self.particle_size)),
                    ..Default::default()
                },
                particle,
            ));
        }
    }

    fn place_particles(&mut self) {
        let bounds = self.half_bounds_size * 0.8;
        for i in 0..self.num_particles {
            let x = -bounds.x + random::<f32>() * bounds.x * 2.0;
            let y = -bounds.y + random::<f32>() * bounds.y * 2.0;
            self.positions[i] = Vec2::new(x, y);
            self.velocities[i] = Vec2::ZERO;
        }
    }

    pub fn update_particles(&mut self, delta: f32) {
        self.update_regions();
        self.calculate_densities();
        if self.frames_to_advance() > 0 {
            self.calculate_pressures(delta);
            self.apply_velocities();
        }
    }

    fn update_spatial_hash(&mut self) {
        for id in 0..self.num_particles {
            // Reset offsets
            self.spatial_offsets[id] = self.num_particles as u32;
            // Update index buffer
            let index = id;
            let cell = get_cell_2d(&self.predicted_positions[index], self.smoothing_radius);
            let hash = hash_cell_2d(&cell);
            let key = key_from_hash(hash, self.num_particles as u32);
            self.spatial_indices[id] = [index as u32, hash, key];
        }
    }

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
            .map(|i| self.density(i))
            .collect();

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

    fn density(&self, id: usize) -> (f32, f32) {
        let position = self.positions[id];
        let mut density = 1.0;

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
                        let distance = (self.positions[*i] - position).length().max(0.000000001);
                        let influence = self.smoothing_kernel(distance);
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
    }

    fn calculate_pressure(&self, id: usize, delta: f32) -> Vec2 {
        let mut velocity = self.velocities[id];
        let pressure_force = self.pressure_force(id) * delta;
        let gravity_force = self.external_forces(&self.positions[id], &velocity) * delta;

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

    pub fn reset(&mut self) {
        self.place_particles();
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

    pub fn toggle_arrows(&mut self) {
        self.debug.show_arrows = !self.debug.show_arrows;
    }

    pub fn toggle_smoothing_radius(&mut self) {
        self.debug.show_smoothing_radius = !self.debug.show_smoothing_radius;
    }

    pub fn toggle_region_grid(&mut self) {
        self.debug.show_region_grid = !self.debug.show_region_grid;
    }

    pub fn toggle_heatmap(&mut self) {
        self.debug.use_heatmap = !self.debug.use_heatmap;
    }

    pub fn toggle_inertia(&mut self) {
        self.debug.use_inertia = !self.debug.use_inertia;
        println!("inertia: {}", self.debug.use_inertia);
    }

    pub fn toggle_viscosity(&mut self) {
        self.debug.use_viscosity = !self.debug.use_viscosity;
        println!("viscosity: {}", self.debug.use_viscosity);
    }

    pub fn log_next_frame(&mut self) {
        self.debug.log_frame = self.debug.current_frame + 1;
    }

    pub fn adj_smoothing_radius(&mut self, increment: f32) {
        self.smoothing_radius = (self.smoothing_radius + increment).max(increment.abs());
        self.smoothing_scaling_factor = 6.0 / (PI * self.smoothing_radius.powf(4.0));
        self.smoothing_derivative_scaling_factor = PI * self.smoothing_radius.powf(4.0) / 6.0;
        println!("smoothing_radius: {}", self.smoothing_radius);
    }

    pub fn adj_gravity(&mut self, increment: f32) {
        self.gravity.y = (self.gravity.y + increment).min(0.0);
        println!("gravity: {}", self.gravity);
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
                            gradient += (Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5) + (Vec2::ZERO - position))
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

    fn smoothing_kernel_poly6(&self, dst: f32) -> f32 {
        if dst < self.smoothing_radius {
            let v: f32 = self.smoothing_radius * self.smoothing_radius - dst * dst;
            return v * v * v * self.poly6_scaling_factor;
        }
        0.0
    }

    fn spiky_kernel_pow3(&self, dst: f32) -> f32 {
        if dst < self.smoothing_radius {
            let v: f32 = self.smoothing_radius - dst;
            return v * v * v * self.spiky_pow3_scaling_factor;
        }
        0.0
    }

    fn spiky_kernel_pow2(&self, dst: f32) -> f32 {
        if dst < self.smoothing_radius {
            let v: f32 = self.smoothing_radius - dst;
            return v * v * self.spiky_pow2_scaling_factor;
        }
        0.0
    }

    fn derivative_spiky_pow3(&self, dst: f32) -> f32 {
        if dst < self.smoothing_radius {
            let v: f32 = self.smoothing_radius - dst;
            return -v * v * self.spiky_pow3_derivative_scaling_factor;
        }
        0.0
    }

    fn derivative_spiky_pow2(&self, dst: f32) -> f32 {
        if dst < self.smoothing_radius {
            let v: f32 = self.smoothing_radius - dst;
            return -v * self.spiky_pow2_derivative_scaling_factor;
        }
        0.0
    }

    fn density_kernel(&self, dst: f32) -> f32 {
        self.spiky_kernel_pow2(dst)
    }

    fn near_density_kernel(&self, dst: f32) -> f32 {
        self.spiky_kernel_pow3(dst)
    }

    fn density_derivative(&self, dst: f32) -> f32 {
        self.derivative_spiky_pow2(dst)
    }

    fn near_density_derivative(&self, dst: f32) -> f32 {
        self.derivative_spiky_pow3(dst)
    }

    fn viscosity_kernel(&self, dst: f32) -> f32 {
        self.smoothing_kernel_poly6(dst)
    }

    fn calculate_density(&self, pos: &Vec2) -> (f32, f32) {
        let origin_cell = get_cell_2d(pos, self.smoothing_radius);
        let sqr_radius = self.smoothing_radius * self.smoothing_radius;
        let mut density = 0.0;
        let mut near_density = 0.0;

        // Neighbour search
        for offset in OFFSETS_2D {
            let cell = (origin_cell.0 + offset.0, origin_cell.1 + offset.1);
            let hash = hash_cell_2d(&cell);
            let key = key_from_hash(hash, self.num_particles as u32);
            let mut curr_index = self.spatial_offsets[key as usize];

            while (curr_index as usize) < self.num_particles {
                let index_data = self.spatial_indices[curr_index as usize];
                curr_index += 1;
                // Exit if no longer looking at correct bin
                if index_data[2] != key {
                    break;
                }
                // Skip if hash does not match
                if index_data[1] != hash {
                    continue;
                }

                let neighbour_index = index_data[0];
                let neighbour_pos = self.predicted_positions[neighbour_index as usize];
                let offset_to_neighbour = neighbour_pos - pos;
                let sqr_dst_to_neighbour = offset_to_neighbour.dot(offset_to_neighbour);

                // Skip if not within radius
                if sqr_dst_to_neighbour > sqr_radius {
                    continue;
                }

                // Calculate density and near density
                let dst = sqr_dst_to_neighbour.sqrt();
                density += self.density_kernel(dst);
                near_density += self.near_density_kernel(dst);
            }
        }

        (density, near_density)
    }

    fn pressure_from_density(&self, density: f32) -> f32 {
        (density - self.target_density) * self.pressure_multiplier
    }

    fn near_pressure_from_density(&self, near_density: f32) -> f32 {
        self.near_pressure_multiplier * near_density
    }

    fn external_forces(&self, pos: &Vec2, velocity: &Vec2) -> Vec2 {
        // Input interactions modify gravity
        if self.interaction_input_strength != 0.0 {
            let input_point_offset = self.interaction_input_point - pos;
            let sqr_dst = input_point_offset.dot(input_point_offset);
            if sqr_dst < self.interaction_input_radius * self.interaction_input_radius {
                let dst = sqr_dst.sqrt();
                let edge_t = dst / self.interaction_input_radius;
                let centre_t = 1.0 - edge_t;
                let dir_to_centre = input_point_offset / dst;

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
