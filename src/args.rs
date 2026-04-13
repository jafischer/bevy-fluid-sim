use clap::Parser;
use once_cell::sync::Lazy;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// A test client for Proactive Voice Moderation
pub struct Args {
    /// Initial window size, as width,height
    #[arg(long, default_value = "900,800")]
    pub win: String,

    /// Number of particles
    #[cfg(debug_assertions)]
    #[arg(long, default_value = "10000")]
    pub num: u32,
    /// Number of particles
    #[cfg(not(debug_assertions))]
    #[arg(long, default_value = "50000")]
    pub num: u32,
    /// Smoothing radius, as a multiple of particle size (e.g. 8.0 = 8x particle diameter).
    #[arg(long, default_value = "10.0")]
    pub smoothing_radius: f32,
    /// Gravity strength
    #[arg(long, default_value = "200.0")]
    pub gravity: f32,
    #[arg(long, default_value = "50000")]
    pub pressure_multiplier: u32,
    #[arg(long, default_value = "5.0")]
    pub viscosity_strength: f32,
    #[arg(long, default_value = "0.5")]
    pub collision_damping: f32,
    /// Radius of the area-of-affect for mouse clicks, as a factor of particle size.
    #[arg(long, default_value = "75")]
    pub interaction_input_radius: u16,
    /// Strength of the attraction/repulsion when mouse is clicked.
    #[arg(long, default_value = "1000.0")]
    pub interaction_input_strength: f32,
    /// Size of the particle sprite, relative to particle size.
    #[arg(long, default_value = "1.0")]
    pub sprite_size: f32,
}

pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);
