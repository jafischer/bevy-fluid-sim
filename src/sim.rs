use std::sync::atomic::Ordering::Relaxed;

use bevy::math::Vec2;
use bevy::prelude::{Component, Mut};

use crate::{Particle, LOG_STUFF};

const GRAVITY: f32 = -9.8;
const PARTICLE_MASS: f32 = 1.0;
const PRESSURE_MULTIPLIER: f32 = 1.0;
const COLLISION_DAMPING: f32 = 0.5;

#[derive(Component, Clone, Debug)]
pub struct Simulation {
    pub smoothing_radius: f32,
    pub smoothing_derivative_scaling_factor: f32,
    pub smoothing_scaling_factor: f32,
    pub target_density: f32,
    pub half_bounds_size: Vec2,
}

impl Simulation {
    pub fn density(&self, pt: &Particle, particle_positions: &Vec<Vec2>) -> f32 {
        let mut density = 1.0; // start off at 1 for self.

        for (i, particle_position) in particle_positions.iter().enumerate() {
            if i == pt.id {
                continue;
            }
            let distance = (particle_position - pt.position).length().max(0.001);
            let influence = self.smoothing_kernel(distance);
            density += PARTICLE_MASS * influence;
        }
        density
    }

    pub fn smoothing_kernel(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            // (self.smoothing_radius - distance) * (self.smoothing_radius - distance) / self.smoothing_scaling_factor
            let scaled_distance = 1.0 - (self.smoothing_radius - distance) / self.smoothing_radius;
            scaled_distance * scaled_distance
        }
    }

    pub fn smoothing_kernel_derivative(&self, distance: f32) -> f32 {
        if distance >= self.smoothing_radius {
            0f32
        } else {
            (distance - self.smoothing_radius) * self.smoothing_derivative_scaling_factor
        }
    }

    pub fn apply_pressure(&self, mut particle: &mut Mut<Particle>, particles: &Vec<Particle>, delta: f32) {
        let is_first_particle = particle.id == 0;
        let pressure_force = self.pressure_force(&particle, &particles);
        let pressure_accel = pressure_force / particle.density;
        let velocity = particle.velocity + (GRAVITY + pressure_accel) * delta;
        if is_first_particle && LOG_STUFF.load(Relaxed) {
            println!(
                "pressure_force: {pressure_force:?} pressure_accel: {pressure_accel:?} velocity: {:?}->{velocity:?}",
                particle.velocity
            );
        }

        // let velocity = particle.velocity;
        let position = particle.position + velocity;
        (particle.position, particle.velocity) = self.resolve_collisions(position, velocity);
    }

    pub fn pressure(&self, density: f32) -> f32 {
        let density_error = density - self.target_density;
        density_error * PRESSURE_MULTIPLIER
    }

    pub fn resolve_collisions(&self, mut position: Vec2, mut velocity: Vec2) -> (Vec2, Vec2) {
        if position.x.abs() > self.half_bounds_size.x {
            position.x = self.half_bounds_size.x * position.x.signum();
            velocity.x *= -1.0 * COLLISION_DAMPING;
        }
        if position.y.abs() > self.half_bounds_size.y {
            position.y = self.half_bounds_size.y * position.y.signum();
            velocity.y *= -1.0 * COLLISION_DAMPING;
        }

        (position, velocity)
    }

    pub fn pressure_force(&self, pt: &Particle, particles: &Vec<Particle>) -> Vec2 {
        let mut gradient = Vec2::default();
        let is_first_particle = pt.id == 0;

        for particle in particles {
            if particle.id == pt.id {
                continue;
            }
            let offset = particle.position - pt.position;
            let distance = offset.length().max(0.001);
            if distance >= self.smoothing_radius {
                continue;
            }
            // Unit vector in the direction of the particle.
            let direction = offset / distance;
            let slope = self.smoothing_kernel_derivative(distance);
            let pressure = self.pressure(particle.density);
            // gradient += pressure * -direction * slope * PARTICLE_MASS / particle.density;
            gradient += pressure * -direction / particle.density;
            if is_first_particle && LOG_STUFF.load(Relaxed) {
                println!("distance:{distance} direction:{direction} slope:{slope} pressure:{pressure} density:{} gradient:{gradient}", particle.density);
            }
        }
        gradient
    }
}
