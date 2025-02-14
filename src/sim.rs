use std::f32::consts::PI;

use bevy::math::Vec2;
use bevy::prelude::{Component, Mut};

use crate::Particle;

const GRAVITY: f32 = -10.0;
const PARTICLE_MASS: f32 = 1.0;
const PRESSURE_MULTIPLIER: f32 = 200.0;
const COLLISION_DAMPING: f32 = 0.5;

#[derive(Component, Clone, Debug)]
pub struct Simulation {
    pub smoothing_radius: f32,
    pub smoothing_derivative_scaling_factor: f32,
    pub smoothing_scaling_factor: f32,
    pub target_density: f32,
    pub half_bounds_size: Vec2,
    pub gravity: Vec2,
}

impl Simulation {
    pub fn new(box_width: f32, box_height: f32, particle_size: f32, scale: f32) -> Simulation {
        let smoothing_radius = 0.2;
        let simulation = Simulation {
            smoothing_radius,
            smoothing_derivative_scaling_factor: PI * smoothing_radius.powf(4.0) / 6.0,
            smoothing_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
            target_density: 250.0,
            half_bounds_size: Vec2::new(box_width, box_height) / 2.0 - particle_size / 2.0,
            gravity: Vec2::new(0.0, GRAVITY * scale),
        };

        println!("{simulation:?}");

        simulation
    }

    pub fn density(&self, pt: &Particle, particle_positions: &Vec<Vec2>) -> f32 {
        let mut density = 0.0;

        for (i, particle_position) in particle_positions.iter().enumerate() {
            if i == pt.id {
                continue;
            }
            let distance = (particle_position - pt.position).length().max(0.000000001);
            let influence = self.smoothing_kernel(distance);
            density += PARTICLE_MASS * influence;
        }
        density
    }

    pub fn smoothing_kernel(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            (self.smoothing_radius - distance) * (self.smoothing_radius - distance) * self.smoothing_scaling_factor
        }
    }

    pub fn smoothing_kernel_derivative(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            (distance - self.smoothing_radius) * self.smoothing_derivative_scaling_factor
        }
    }

    pub fn apply_pressure(&self, particle: &mut Mut<Particle>, particles: &Vec<Particle>, delta: f32) {
        let pressure_force = self.pressure_force(&particle, &particles);
        particle.velocity += (self.gravity + pressure_force) * delta;
        particle.position = particle.position + particle.velocity;
        self.resolve_collisions(particle);
    }

    pub fn pressure(&self, density: f32) -> f32 {
        let density_error = density - self.target_density;
        density_error * PRESSURE_MULTIPLIER
    }

    pub fn resolve_collisions(&self, particle: &mut Mut<Particle>) {
        if particle.position.x.abs() > self.half_bounds_size.x {
            particle.position.x = self.half_bounds_size.x * particle.position.x.signum();
            particle.velocity.x *= -1.0 * COLLISION_DAMPING;
        }
        if particle.position.y.abs() > self.half_bounds_size.y {
            particle.position.y = self.half_bounds_size.y * particle.position.y.signum();
            particle.velocity.y *= -1.0 * COLLISION_DAMPING;
        }
    }

    pub fn pressure_force(&self, pt: &Particle, particles: &Vec<Particle>) -> Vec2 {
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
            let pressure = self.pressure(particle.density);
            gradient += direction * slope * pressure / particle.density;
        }
        gradient // / pt.density
    }
}
