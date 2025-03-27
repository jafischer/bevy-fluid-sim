use bevy::prelude::*;

use crate::sim_struct::Simulation;
use crate::spatial_hash::*;

impl Simulation {
    //
    // These functions are translated from Sebastian's Fluid-Sim shader code (hence the
    // sfs_ prefix), but I've not yet switched to using it.
    //

    pub fn sfs_smoothing_kernel_poly6(&self, distance: f32) -> f32 {
        if distance < self.smoothing_radius {
            let v: f32 = self.smoothing_radius * self.smoothing_radius - distance * distance;
            return v * v * v * self.poly6_scaling_factor;
        }
        0.0
    }

    pub fn sfs_spiky_kernel_pow3(&self, distance: f32) -> f32 {
        if distance < self.smoothing_radius {
            let v: f32 = self.smoothing_radius - distance;
            return v * v * v * self.spiky_pow3_scaling_factor;
        }
        0.0
    }

    pub fn sfs_spiky_kernel_pow2(&self, distance: f32) -> f32 {
        if distance < self.smoothing_radius {
            let v: f32 = self.smoothing_radius - distance;
            return v * v * self.spiky_pow2_scaling_factor;
        }
        0.0
    }

    pub fn sfs_derivative_spiky_pow3(&self, distance: f32) -> f32 {
        if distance < self.smoothing_radius {
            let v: f32 = self.smoothing_radius - distance;
            return -v * v * self.spiky_pow3_derivative_scaling_factor;
        }
        0.0
    }

    pub fn sfs_derivative_spiky_pow2(&self, distance: f32) -> f32 {
        if distance < self.smoothing_radius {
            let v: f32 = self.smoothing_radius - distance;
            return -v * self.spiky_pow2_derivative_scaling_factor;
        }
        0.0
    }

    pub fn sfs_density_kernel(&self, distance: f32) -> f32 {
        self.sfs_spiky_kernel_pow2(distance)
    }

    pub fn sfs_near_density_kernel(&self, distance: f32) -> f32 {
        self.sfs_spiky_kernel_pow3(distance)
    }

    pub fn sfs_density_derivative(&self, distance: f32) -> f32 {
        self.sfs_derivative_spiky_pow2(distance)
    }

    pub fn sfs_near_density_derivative(&self, distance: f32) -> f32 {
        self.sfs_derivative_spiky_pow3(distance)
    }

    pub fn sfs_viscosity_kernel(&self, distance: f32) -> f32 {
        self.sfs_smoothing_kernel_poly6(distance)
    }

    pub fn sfs_calculate_density(&self, pos: &Vec2) -> (f32, f32) {
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
                let sqr_distance_to_neighbour = offset_to_neighbour.dot(offset_to_neighbour);

                // Skip if not within radius
                if sqr_distance_to_neighbour > sqr_radius {
                    continue;
                }

                // Calculate density and near density
                let distance = sqr_distance_to_neighbour.sqrt();
                density += self.sfs_density_kernel(distance);
                near_density += self.sfs_near_density_kernel(distance);
            }
        }

        (density, near_density)
    }
    pub fn sfs_calculate_pressure_force(&self, id: usize) -> Vec2 {
        let density = self.densities[id].0;
        let density_near = self.densities[id].1;
        let pressure = self.sfs_pressure_from_density(density);
        let near_pressure = self.sfs_near_pressure_from_density(density_near);
        let mut pressure_force = Vec2::default();

        let pos = self.predicted_positions[id];
        let origin_cell = get_cell_2d(&pos, self.smoothing_radius);

        for i in 0..9 {
            let hash = hash_cell_2d(&(origin_cell.0 + OFFSETS_2D[i].0, origin_cell.1 + OFFSETS_2D[i].1));
            let key = key_from_hash(hash, self.num_particles as u32);
            let mut curr_index = self.spatial_offsets[key as usize];

            while curr_index < self.num_particles as u32 {
                let neighbour_index = curr_index as usize;
                curr_index += 1;

                if neighbour_index == id {
                    continue;
                }

                let neighbour_key = self.spatial_keys[neighbour_index];
                if neighbour_key != key {
                    break;
                }

                let neighbour_pos = self.predicted_positions[neighbour_index];
                let offset_to_neighbour = neighbour_pos - pos;
                let distance = offset_to_neighbour.length();

                if distance > self.smoothing_radius {
                    continue;
                }

                let dir_to_neighbour =
                    if distance > 0.0 { offset_to_neighbour / distance } else { Vec2::new(0.0, 1.0) };

                let neighbour_density = self.densities[neighbour_index].0;
                let neighbour_near_density = self.densities[neighbour_index].1;
                let neighbour_pressure = self.sfs_pressure_from_density(neighbour_density);
                let neighbour_near_pressure = self.sfs_near_pressure_from_density(neighbour_near_density);

                let shared_pressure = (pressure + neighbour_pressure) * 0.5;
                let shared_near_pressure = (near_pressure + neighbour_near_pressure) * 0.5;

                pressure_force +=
                    dir_to_neighbour * self.sfs_density_derivative(distance) * shared_pressure / neighbour_density;
                pressure_force += dir_to_neighbour * self.sfs_near_density_derivative(distance) * shared_near_pressure
                    / neighbour_near_density;
            }
        }

        pressure_force / density
    }

    pub fn sfs_calculate_viscosity(&mut self, id: usize, delta_time: f32) {
        let pos = self.predicted_positions[id];
        let origin_cell = get_cell_2d(&pos, self.smoothing_radius);

        let mut viscosity_force = Vec2::default();
        let velocity = self.velocities[id];

        for i in 0..9 {
            let hash = hash_cell_2d(&(origin_cell.0 + OFFSETS_2D[i].0, origin_cell.1 + OFFSETS_2D[i].1));
            let key = key_from_hash(hash, self.num_particles as u32);
            let mut curr_index = self.spatial_offsets[key as usize];

            while curr_index < self.num_particles as u32 {
                let neighbour_index = curr_index as usize;
                curr_index += 1;

                if neighbour_index == id {
                    continue;
                }

                let neighbour_key = self.spatial_keys[neighbour_index];
                if neighbour_key != key {
                    break;
                }

                let neighbour_pos = self.predicted_positions[neighbour_index];
                let offset_to_neighbour = neighbour_pos - pos;
                let distance = offset_to_neighbour.length();
                if distance > self.smoothing_radius {
                    continue;
                }
                let neighbour_velocity = self.velocities[neighbour_index];
                viscosity_force += (neighbour_velocity - velocity) * self.sfs_viscosity_kernel(distance);
            }
        }

        self.velocities[id] += viscosity_force * self.viscosity_strength * delta_time;
    }

    pub fn sfs_pressure_from_density(&self, density: f32) -> f32 {
        (density - self.target_density) * self.pressure_multiplier
    }

    pub fn sfs_near_pressure_from_density(&self, near_density: f32) -> f32 {
        self.near_pressure_multiplier * near_density
    }

    pub fn sfs_update_spatial_hash(&mut self) {
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
}
