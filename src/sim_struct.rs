use std::fmt::{Debug, Formatter};
use bevy::math::Vec2;
use bevy::prelude::Component;

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
    pub prediction_factor: f32,
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
    pub use_sfs: bool,
    pub current_frame: u32,
    pub frames_to_show: u32,
    pub log_frame: u32,
    pub show_fps: bool,
    pub show_smoothing_radius: bool,
    pub show_region_grid: bool,
    pub use_inertia: bool,
    pub use_viscosity: bool,
    pub use_heatmap: bool,
}
