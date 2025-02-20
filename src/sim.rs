use std::f32::consts::PI;

use bevy::prelude::*;
use rand::random;

use crate::Particle;

#[derive(Component, Clone, Debug)]
pub struct Simulation {
    pub smoothing_radius: f32,
    pub smoothing_scaling_factor: f32,
    pub smoothing_derivative_scaling_factor: f32,
    pub num_particles: u32,
    pub particle_size: f32,
    pub scale: f32,
    pub grid_size: f32,
    pub rows: u32,
    pub cols: u32,
    pub half_bounds_size: Vec2,
    pub gravity: Vec2,
    pub target_density: f32,
    pub pressure_multiplier: f32,
    pub collision_damping: f32,
    debug: DebugParams,
}

#[derive(Clone, Debug)]
struct DebugParams {
    current_frame: u32,
    frames_to_show: u32,
    log_frame: u32,
    show_arrows: bool,
    use_gravity: bool,
}

impl Simulation {
    pub fn new(window_width: f32, window_height: f32) -> Simulation {
        let fluid_h = window_height * 0.67;
        let num_particles = 1000;
        let (grid_size, cols, rows) = Self::subdivide_into_squares(window_width, fluid_h, num_particles);

        // Because the kernel math blows up with smoothing radius values > 1, we don't want to use the
        // actual window coordinates. In Sebastian's video, at 5:40, he shows a smoothing radius of 0.5
        // that is about 12 particles wide:
        // (https://youtu.be/rSKMYc1CQHE?si=3sibErk0e4CYC5wF&t=340)
        // So we want the grid size to be scaled down to 0.08333 (1/12).
        // grid_size * scale = 0.08333
        // scale = 0.08333 / grid_size
        let scale = 0.08333 / grid_size;
        let grid_size = grid_size * scale;
        let particle_size = grid_size * 0.5;
        let smoothing_radius = 0.2;
        let simulation = Simulation {
            smoothing_radius,
            // This is SpikyPow2ScalingFactor in Fluid-Sim.
            smoothing_scaling_factor: 6.0 / (PI * smoothing_radius.powf(4.0)),
            // Why doesn't this version (corresponding to SpikyPow2DerivativeScalingFactor in
            // Fluid-Sim) work?
            // smoothing_derivative_scaling_factor: 12.0 / (PI * smoothing_radius.powf(4.0)),
            // And where did I get this from? --> There is no occurrence in Fluid-Sim of:
            //     Mathf.PI * Mathf.pow(smoothingRadius, 4) / 6
            // ... maybe I just transcribed an early version of this calculation
            // from a screenshot in the video.
            smoothing_derivative_scaling_factor: PI * smoothing_radius.powf(4.0) / 6.0,
            num_particles,
            particle_size,
            scale,
            grid_size,
            rows,
            cols,
            half_bounds_size: Vec2::new(window_width, window_height) * scale / 2.0 - particle_size / 2.0,
            gravity: Vec2::new(0.0, -1.0 * scale),
            target_density: 200.0,
            pressure_multiplier: 500.0,
            collision_damping: 0.25,
            debug: DebugParams {
                current_frame: 0,
                frames_to_show: u32::MAX,
                log_frame: 0,
                show_arrows: false,
                use_gravity: true,
            },
        };

        println!("{simulation:?}");

        simulation
    }

    pub fn spawn_particles(
        &self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) {
        let color = Color::linear_rgb(0.0, 0.3, 1.0);
        let scaled_width = self.grid_size * self.cols as f32;
        let scaled_height = self.grid_size * self.rows as f32;

        let mut id = 0;
        let x_start = -scaled_width / 2.0;
        let y_start = -scaled_height / 2.0;
        for r in 0..self.rows {
            for c in 0..self.cols {
                // If we go back to random placement, we can get rid of grid_size, rows, cols, etc.
                // let x = x_start + random::<f32>() * scaled_width;
                // let y = y_start + random::<f32>() * scaled_height;
                let x = x_start + (c as f32 + 0.5) * self.grid_size + random::<f32>() * self.grid_size
                    - self.grid_size / 2.0;
                let y = y_start + scaled_height - (r as f32 + 0.5) * self.grid_size + random::<f32>() * self.grid_size
                    - self.grid_size / 2.0;
                // let velocity = Vec2::new(random::<f32>() * 1.0 - 0.5, random::<f32>() * 1.0 - 0.5) * particle_size / 2.0;
                let velocity = Vec2::ZERO;
                commands.spawn((
                    Mesh2d(meshes.add(Circle {
                        radius: self.particle_size / 2.0,
                    })),
                    MeshMaterial2d(materials.add(color)),
                    Transform::from_translation(Vec3 { x, y, z: 0.0 }),
                    Particle {
                        id,
                        position: Vec2 { x, y },
                        velocity,
                        density: 0.0,
                    },
                ));
                id += 1;
            }
        }
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

    pub fn show_arrows(&self) -> bool {
        self.debug.show_arrows
    }

    pub fn toggle_arrows(&mut self) {
        self.debug.show_arrows = !self.debug.show_arrows;
    }

    pub fn toggle_gravity(&mut self) {
        self.debug.use_gravity = !self.debug.use_gravity;
    }

    pub fn log_next_frame(&mut self) {
        self.debug.log_frame = self.debug.current_frame + 1;
    }

    pub fn inc_smoothing_radius(&mut self) {
        self.smoothing_radius += 0.1;
        self.smoothing_scaling_factor = 6.0 / (PI * self.smoothing_radius.powf(4.0));
        self.smoothing_derivative_scaling_factor = PI * self.smoothing_radius.powf(4.0) / 6.0;
        println!("smoothing_radius: {}", self.smoothing_radius);
        self.log_next_frame();
    }

    pub fn dec_smoothing_radius(&mut self) {
        if self.smoothing_radius > 0.1 {
            self.smoothing_radius -= 0.1;
            self.smoothing_scaling_factor = 6.0 / (PI * self.smoothing_radius.powf(4.0));
            self.smoothing_derivative_scaling_factor = PI * self.smoothing_radius.powf(4.0) / 6.0;
            println!("smoothing_radius: {}", self.smoothing_radius);
            self.log_next_frame();
        }
    }

    pub fn end_frame(&mut self) {
        self.debug.current_frame += 1;
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

    pub fn calculate_pressure(&self, particle: &mut Mut<Particle>, particles: &Vec<Particle>, delta: f32) {
        let pressure_force = self.pressure_force(&particle, &particles);
        if self.debug.use_gravity {
            particle.velocity += (self.gravity + pressure_force) * delta;
        } else {
            particle.velocity = pressure_force * delta;
        }
    }

    pub fn apply_velocity(&self, particle: &mut Mut<Particle>) {
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

    /// Divides a rectangular region into (roughly) n squares.
    ///
    /// Got it from ChatGPT, but as usual even this straightforward function had errors that
    /// I had to fix...
    fn subdivide_into_squares(w: f32, h: f32, n: u32) -> (f32, u32, u32) {
        // Step 1: Calculate the target area of each square
        let target_area = (w * h) / n as f32;

        // Step 2: Calculate the side length of each square
        let side_length = target_area.sqrt();

        // Step 3: Calculate the number of columns and rows
        let columns = w / side_length;
        let rows = n as f32 / columns;

        // Step 4: Adjust the final side length to fit evenly
        let side_length = f32::min(w / columns, h / rows);

        (side_length, columns as u32, rows as u32)
    }
}
