use std::f32::consts::PI;

use bevy::prelude::Vec2;

use crate::sim_struct::Simulation;

impl Simulation {
    pub fn reset(&mut self) {
        self.place_particles();
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
        self.debug.use_heatmap = !self.debug.use_heatmap;
    }

    pub fn toggle_inertia(&mut self) {
        self.debug.use_inertia = !self.debug.use_inertia;
    }

    pub fn reset_inertia(&mut self) {
        (0..self.num_particles).for_each(|i| self.velocities[i] = Vec2::splat(0.0));
    }

    pub fn toggle_viscosity(&mut self) {
        self.debug.use_viscosity = !self.debug.use_viscosity;
    }

    pub fn log_next_frame(&mut self) {
        self.debug.log_frame = self.debug.current_frame + 1;
    }

    pub fn adj_smoothing_radius(&mut self, increment: f32) {
        self.smoothing_radius = (self.smoothing_radius + increment).max(increment.abs());
        self.smoothing_scaling_factor = 6.0 / (PI * self.smoothing_radius.powf(4.0));
        self.smoothing_derivative_scaling_factor = PI * self.smoothing_radius.powf(4.0) / 6.0;
    }

    pub fn adj_gravity(&mut self, increment: f32) {
        self.gravity.y = (self.gravity.y + increment).min(0.0);
    }
}
