use bevy::prelude::Vec2;

use crate::sim_struct::Simulation;

impl Simulation {
    pub fn reset(&mut self) {
        self.place_particles();

        self.min_velocity = f32::MAX;
        self.max_velocity = 0f32;
        self.min_density = f32::MAX;
        self.max_density = 0f32;
    }

    pub fn frames_to_advance(&self) -> u32 {
        self.debug.frames_to_show
    }

    pub fn set_frames_to_show(&mut self, val: u32) {
        self.debug.frames_to_show = val;
    }

    pub fn toggle_smoothing_radius(&mut self) {
        self.debug.show_smoothing_radius = !self.debug.show_smoothing_radius;
    }

    pub fn toggle_region_grid(&mut self) {
        self.debug.show_region_grid = !self.debug.show_region_grid;
    }

    pub fn toggle_fps(&mut self) {
        self.debug.show_fps = !self.debug.show_fps;
    }

    pub fn toggle_heatmap(&mut self) {
        self.debug.density_heatmap = !self.debug.density_heatmap;
    }

    pub fn reset_inertia(&mut self) {
        (0..self.num_particles).for_each(|i| self.velocities[i] = Vec2::splat(0.0));
    }

    pub fn toggle_arrows(&mut self) {
        self.debug.show_arrows = !self.debug.show_arrows;
    }

    pub fn toggle_predicted(&mut self) {
        self.debug.use_predicted_positions = !self.debug.use_predicted_positions;
    }

    pub fn log_next_frame(&mut self) {
        self.debug.log_frame = self.debug.current_frame + 1;
    }

    pub fn adj_smoothing_radius(&mut self, increment: f32) {
        let smoothing_radius = self.smoothing_radius / self.particle_size;

        self.set_smoothing_radius((smoothing_radius + increment).max(increment.abs()));
    }

    pub fn adj_gravity(&mut self, increase: bool) {
        self.gravity.y = if increase { self.gravity.y * 1.10 } else { self.gravity.y / 1.10 };
    }

    pub fn adj_pressure(&mut self, increase: bool) {
        self.pressure_multiplier =
            if increase { self.pressure_multiplier * 1.10 } else { self.pressure_multiplier / 1.10 };
    }

    pub fn adj_viscosity(&mut self, increase: bool) {
        self.viscosity_strength =
            if increase { self.viscosity_strength * 1.10 } else { self.viscosity_strength / 1.10 };
    }
}
