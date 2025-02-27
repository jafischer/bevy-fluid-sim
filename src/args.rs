use clap::Parser;
use once_cell::sync::Lazy;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// A test client for Proactive Voice Moderation
pub struct Args {
    /// Initial window size, as width,height
    #[arg(long, default_value = "400,500")]
    pub win: String,

    /// Number of particles
    #[arg(long, default_value = "2500")]
    pub num: u32,
    /// Initial smoothing radius (where 1.0 == a circle of approx 25 particles in diameter).
    #[arg(long, default_value = "0.3")]
    pub smoothing_radius: f32,
    /// Initial gravity
    #[arg(long, default_value = "-10.0")]
    pub gravity: f32,
    #[arg(long, default_value = "500")]
    pub pressure_multiplier: u32,
    #[arg(long, default_value = "25")]
    pub near_pressure_multiplier: u32,
    #[arg(long, default_value = "0.25")]
    pub collision_damping: f32,
    /// Radius of the area-of-affect for mouse clicks, as a number of particles.
    #[arg(long, default_value = "20")]
    pub interaction_input_radius: u16,
}

pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);
