use std::fmt::{Debug, Formatter};

use bevy::math::Vec2;
use bevy::prelude::Component;

#[derive(Component)]
pub struct Simulation {
    pub smoothing_radius: f32,
    pub smoothing_scaling_factor: f32,
    pub smoothing_derivative_scaling_factor: f32,
    pub viscosity_scaling_factor: f32,
    pub num_particles: usize,
    pub particle_size: f32,
    pub half_bounds_size: Vec2,
    pub target_density: f32,

    // Adjustable parameters
    pub gravity: Vec2,
    pub pressure_multiplier: f32,
    pub viscosity_strength: f32,
    pub collision_damping: f32,
    pub speed: f32,
    pub sprite_size: f32,
    pub interaction_input_strength: f32,
    pub interaction_input_radius: f32,

    // Particle information:
    pub positions: Vec<Vec2>,
    pub predicted_positions: Vec<Vec2>,
    pub velocities: Vec<Vec2>,
    pub densities: Vec<f32>,
    pub region_rows: usize,
    pub region_cols: usize,
    pub regions: Vec<Vec<Vec<usize>>>,
    pub interaction_input_point: Option<Vec2>,
    pub min_velocity: f32,
    pub max_velocity: f32,
    pub min_density: f32,
    pub max_density: f32,

    pub debug: DebugParams,
}

impl Debug for Simulation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Simulation:")?;
        writeln!(
            f,
            "    smoothing_radius: {} ({})",
            self.smoothing_radius,
            self.smoothing_radius / self.particle_size
        )?;
        writeln!(f, "    num_particles: {}", self.num_particles)?;
        writeln!(f, "    particle_size: {}", self.particle_size)?;
        writeln!(f, "    gravity: {} ({})", self.gravity.y, self.gravity.y / self.particle_size)?;
        writeln!(f, "    target_density: {}", self.target_density)?;
        writeln!(
            f,
            "    pressure_multiplier: {} ({})",
            self.pressure_multiplier,
            self.pressure_multiplier / self.particle_size
        )?;
        writeln!(f, "    viscosity_strength: {}", self.viscosity_strength)?;
        writeln!(f, "    collision_damping: {}", self.collision_damping)
    }
}

pub struct DebugParams {
    pub current_frame: u32,
    pub frames_to_show: u32,
    pub log_frame: u32,
    pub show_fps: bool,
    pub show_smoothing_radius: bool,
    pub show_region_grid: bool,
    pub density_heatmap: bool,
    pub show_arrows: bool,
    pub use_predicted_positions: bool,
}
