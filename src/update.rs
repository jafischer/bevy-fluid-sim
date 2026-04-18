use std::ops::{Deref, DerefMut};
use std::sync::Mutex;

use bevy::color::Color;
use bevy::color::palettes::basic::{GRAY, LIME, YELLOW};
use bevy::math::{Vec2, Vec3, Vec3Swizzles};
use bevy::prelude::{Commands, Entity, Gizmos, Query, Res, Single, Sprite, Text, Time, Transform};
use once_cell::sync::Lazy;

use crate::SpriteImage;
use crate::components::*;
use crate::sim_struct::Simulation;

// Some color definitions for blending.
const COLD: Vec3 = Vec3::new(0.0, 0.0, 0.6);
const HOT: Vec3 = Vec3::new(1.0, 0.2, 0.2);

const STOPPED: Vec3 = Vec3::new(0.1, 0.1, 0.5);
const FAST: Vec3 = Vec3::new(0.9, 1.0, 0.0);

static TOT_FPS: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.0));

/// Performs one step of the simulation, and draws the particles.
/// Chooses a color based on the particle's density or velocity, depending on the settings.
pub fn update_particles(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut Particle)>,
    // time: Res<Time>,
    mut sim: Single<&mut Simulation>,
    sprite_image: Single<&SpriteImage>,
) {
    // I'm using a fixed delta of 1/60th of a second rather than relying on time.delta_secs()), to avoid the
    // chaos that can arise from sudden framerate pauses.
    sim.update_particles(1.0 / 60.0); // time.delta_secs());

    let custom_size = Some(Vec2::splat(sim.particle_size * sim.sprite_size));

    particle_query.iter_mut().for_each(|(entity, mut transform, particle)| {
        transform.translation.x = sim.positions[particle.id].x;
        transform.translation.y = sim.positions[particle.id].y;

        let color = if sim.debug.show_arrows {
            Color::linear_rgba(0.0, 0.0, 0.0, 0.)
        } else if particle.watched {
            Color::linear_rgb(1.0, 1.0, 0.0)
        } else if sim.debug.density_heatmap {
            let density_ratio = (sim.densities[particle.id] - sim.min_density) / (sim.max_density - sim.min_density);
            let density_scale = density_ratio.powf(2.0);
            let rgb = COLD + density_scale * (HOT - COLD);
            Color::linear_rgb(rgb.x, rgb.y, rgb.z)
        } else {
            let speed_ratio = sim.velocities[particle.id].length() / sim.max_velocity;
            let speed_scale = speed_ratio.powf(1.0 / 4.0);
            let rgb = STOPPED + speed_scale * (FAST - STOPPED);
            Color::linear_rgb(rgb.x, rgb.y, rgb.z)
        };

        commands.entity(entity).insert(Sprite {
            image: sprite_image.handle.clone(),
            custom_size,
            color,
            ..Default::default()
        });
    });

    sim.end_frame();
}

pub fn update_fps(mut query: Query<(&mut Text, &FpsText)>, time: Res<Time>, sim: Single<&Simulation>) {
    for (mut span, _) in &mut query {
        if time.delta_secs() == 0.0 {
            return;
        }

        let cur_fps = 1.0 / time.delta_secs();
        let mut tot_fps = TOT_FPS.lock().unwrap();

        *tot_fps.deref_mut() += cur_fps;
        if sim.debug.show_fps {
            **span = format!("FPS: {:5.1} / avg {:.1}", cur_fps, tot_fps.deref() / (sim.debug.current_frame as f32));
        } else if !span.is_empty() {
            span.clear();
        }
    }
}

pub fn draw_debug_info(
    mut gizmos: Gizmos,
    sim: Single<&Simulation>,
    particle_query: Query<(&mut Transform, &mut Particle)>,
) {
    if sim.debug.show_arrows {
        particle_query.iter().for_each(|(transform, particle)| {
            let arrow_end = transform.translation.xy() + sim.velocities[particle.id] * 1. / 60. * sim.speed;
            gizmos
                .arrow(transform.translation.xy().extend(0.0), arrow_end.extend(0.0), YELLOW)
                .with_tip_length(sim.particle_size);
        });
    }
    if sim.debug.show_smoothing_radius {
        gizmos.circle_2d(sim.positions[0], sim.smoothing_radius, LIME);
    }
    if sim.debug.show_region_grid {
        let bottom = -sim.half_bounds_size.y;
        let left = -sim.half_bounds_size.x;
        for row in 0..sim.region_rows {
            for col in 0..sim.region_cols {
                gizmos.rect_2d(
                    Vec2::new(left + col as f32 * sim.smoothing_radius, bottom + row as f32 * sim.smoothing_radius),
                    Vec2::splat(sim.smoothing_radius),
                    GRAY,
                );
            }
        }
    }
}
