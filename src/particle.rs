use bevy::math::Vec2;
use bevy::prelude::Component;

#[derive(Component, Clone, Debug)]
pub struct Particle {
    pub id: usize,
    pub position: Vec2,
    pub velocity: Vec2,
    pub density: f32,
}
