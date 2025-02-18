use bevy::math::Vec2;
use bevy::prelude::{Component, Mut};
use std::f32::consts::PI;

use crate::Particle;

#[derive(Component, Clone, Debug)]
pub struct Simulation {
    pub smoothing_radius: f32,
    pub smoothing_scaling_factor: f32,
    pub smoothing_derivative_scaling_factor: f32,
    pub half_bounds_size: Vec2,
    pub gravity: Vec2,
    pub target_density: f32,
    pub pressure_multiplier: f32,
    pub collision_damping: f32,
    debug: DebugParams,
}

#[derive(Clone, Debug)]
struct DebugParams {
    frames_to_show: u32,
    log_this_frame: bool,
    show_arrows: bool,
}

impl Simulation {
    pub fn new(box_width: f32, box_height: f32, particle_size: f32, scale: f32) -> Simulation {
        let smoothing_radius = 0.2;
        let simulation = Simulation {
            smoothing_radius,
            // SpikyPow2ScalingFactor: 6 / (Mathf.PI * Mathf.Pow(smoothingRadius, 4))
            smoothing_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
            // SpikyPow2DerivativeScalingFactor: 12 / (Mathf.Pow(smoothingRadius, 4) * Mathf.PI)
            // smoothing_derivative_scaling_factor: 12.0 / (PI * smoothing_radius.powf(4.0)),
            smoothing_derivative_scaling_factor: PI * smoothing_radius.powf(4.0) / 6.0,
            half_bounds_size: Vec2::new(box_width, box_height) / 2.0 - particle_size / 2.0,
            gravity: Vec2::new(0.0, -1.0 * scale),
            target_density: 100.0,
            pressure_multiplier: 500.0,
            collision_damping: 0.5,
            debug: DebugParams {
                frames_to_show: 0,
                log_this_frame: false,
                show_arrows: false,
            },
        };

        println!("{simulation:?}");

        simulation
    }

    pub fn frames_to_show(&self) -> u32 {
        self.debug.frames_to_show
    }

    pub fn set_frames_to_show(&mut self, val: u32) {
        self.debug.frames_to_show = val;
    }

    pub fn log_this_frame(&self) -> bool {
        self.debug.log_this_frame
    }

    pub fn show_arrows(&self) -> bool {
        self.debug.show_arrows
    }

    pub fn end_frame(&mut self) {
        self.debug.log_this_frame = false;
    }

    pub fn toggle_show_arrows(&mut self) {
        self.debug.show_arrows = !self.debug.show_arrows;
    }

    pub fn log_next_frame(&mut self) {
        self.debug.log_this_frame = true;
    }

    pub fn density(&self, pt: &Particle, particle_positions: &Vec<Vec2>) -> f32 {
        let mut density = 0.0;

        for (i, particle_position) in particle_positions.iter().enumerate() {
            if i == pt.id {
                continue;
            }
            let distance = (particle_position - pt.position).length().max(0.000000001);
            let influence = self.smoothing_kernel(distance);
            density += influence;
        }
        density
    }

    fn smoothing_kernel(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            // float v = radius - dst;
            // return v * v * SpikyPow2ScalingFactor;
            (self.smoothing_radius - distance) * (self.smoothing_radius - distance) * self.smoothing_scaling_factor
        }
    }

    fn smoothing_kernel_derivative(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            // float v = radius - dst;
            // return -v * SpikyPow2DerivativeScalingFactor;
            (distance - self.smoothing_radius) * self.smoothing_derivative_scaling_factor
        }
    }

    pub fn apply_pressure(&self, particle: &mut Mut<Particle>, particles: &Vec<Particle>, delta: f32) {
        let pressure_force = self.pressure_force(&particle, &particles);
        // particle.velocity += (self.gravity + pressure_force) * delta;
        particle.velocity = pressure_force * delta;
        particle.position = particle.position + particle.velocity;
        self.resolve_collisions(particle);
    }

    fn shared_pressure(&self, density1: f32, density2: f32) -> f32 {
        let density_error1 = density1 - self.target_density;
        let density_error2 = density2 - self.target_density;
        (density_error1 + density_error2) * self.pressure_multiplier / 2.0
    }

    fn resolve_collisions(&self, particle: &mut Mut<Particle>) {
        if particle.position.x.abs() > self.half_bounds_size.x {
            particle.position.x = self.half_bounds_size.x * particle.position.x.signum();
            particle.velocity.x *= -1.0 * self.collision_damping;
        }
        if particle.position.y.abs() > self.half_bounds_size.y {
            particle.position.y = self.half_bounds_size.y * particle.position.y.signum();
            particle.velocity.y *= -1.0 * self.collision_damping;
        }
    }

    fn pressure_force(&self, pt: &Particle, particles: &Vec<Particle>) -> Vec2 {
        let mut gradient = Vec2::default();

        for particle in particles {
            if particle.id == pt.id {
                continue;
            }
            let offset = particle.position - pt.position;
            let distance = offset.length().max(0.000000001);
            if distance >= self.smoothing_radius {
                continue;
            }
            // Unit vector in the direction of the particle.
            let direction = offset / distance;
            let slope = self.smoothing_kernel_derivative(distance);
            let pressure = self.shared_pressure(pt.density, particle.density);
            // pressureForce += dirToNeighbour * DensityDerivative(dst, smoothingRadius) * sharedPressure / neighbourDensity;
            gradient += direction * slope * pressure / particle.density;
        }
        gradient // / pt.density
    }
}
