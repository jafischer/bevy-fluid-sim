use bevy::math::Vec2;
use bevy::prelude::Component;

#[derive(Component, Clone, Debug, Default)]
pub struct Particle {
    pub id: usize,
}
