use clap::Parser;
use once_cell::sync::Lazy;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// A test client for Proactive Voice Moderation
pub struct Args {
    /// Initial window size, as width,height
    #[arg(long, default_value = "600,800")]
    pub win: String,

    #[cfg(debug_assertions)]
    /// Number of particles
    #[arg(long, default_value = "10000")]
    pub num: u32,
    #[cfg(not(debug_assertions))]
    /// Number of particles
    #[arg(long, default_value = "50000")]
    pub num: u32,
    /// Initial smoothing radius (where 1.0 == a circle of approx 25 particles in diameter).
    #[arg(long, default_value = "0.33")]
    pub smoothing_radius: f32,
    /// Initial gravity strength
    #[arg(long, default_value = "1.5")]
    pub gravity: f32,
    #[arg(long, default_value = "1000")]
    pub pressure_multiplier: u32,
    // #[arg(long, default_value = "25")]
    // pub near_pressure_multiplier: u32,
    #[arg(long, default_value = "0.5")]
    pub collision_damping: f32,
    /// Radius of the area-of-affect for mouse clicks, as a factor of particle size.
    #[arg(long, default_value = "30")]
    pub interaction_input_radius: u16,
    /// Strength of the attraction/repulsion when mouse is clicked.
    #[arg(long, default_value = "200.0")]
    pub interaction_input_strength: f32,
    /// Size of the particle sprite, relative to particle size.
    #[arg(long, default_value = "2.0")]
    pub sprite_size: f32,
    /// Speed limit (for fake viscosity), as a factor of particle size.
    #[arg(long, default_value = "30.0")]
    pub speed_limit: f32,
}

pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);
