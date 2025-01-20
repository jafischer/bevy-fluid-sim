use std::f32::consts::PI;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use once_cell::sync::Lazy;
use rand::{random, Rng};

const NUM_PARTICLES: u32 = 5000;
const GRAVITY: f32 = 9.8;
const PARTICLE_MASS: f32 = 1.0;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins,))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

#[derive(Component)]
struct MyCameraMarker;

#[derive(Component)]
struct Particle {
    position: Vec2,
    velocity: Vec2,
}

static rng: Lazy<Rng> = Lazy::new(|| Rng::)

fn setup(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window = window_query.get_single().unwrap();
    let fluid_h = window.height() * 0.67;
    let (grid_size, cols, rows) = subdivide_into_squares(window.width(), fluid_h, NUM_PARTICLES);
    let particle_size = grid_size * 0.8;
    let color = Color::linear_rgb(0.0, 0.3, 1.0);

    commands.spawn((
        Camera2d,
        Transform::from_xyz(
            window.width() / 2.0,
            window.height() / 2.0,
            0.0)
    ));

    for r in 0..rows {
        for c in 0..cols {
            let x = (c as f32 + 0.5) * grid_size;
            let y = window.height() - (r as f32 + 0.5) * grid_size;
            commands.spawn((
                Mesh2d(meshes.add(Circle {
                        radius: particle_size / 2.0,
                    })),
                    MeshMaterial2d(materials.add(color)),
                    Transform::from_translation(Vec3 { x, y, z: 0.0 }),
                Particle {
                    position: Vec2::new(x, y),
                    velocity: Vec2::new(random::<f32>(), random::<f32>()),
                },
            ));
        }
    }
}

fn update(particle_query: Query<&Particle>) {

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
    let columns = (w / side_length) as u32;
    let rows = (n as f32 / columns as f32) as u32;

    // Step 4: Adjust the final side length to fit evenly
    let side_length = f32::min(w / columns as f32, h / rows as f32);

    (side_length, columns, rows)
}

fn smoothing_kernel(radius: f32, distance: f32) -> f32 {
    let volume = PI * radius.powf(8.0) / 4.0;
    let value = (radius * radius - distance * distance).max(0.0);
    value * value * value / volume
}

fn density(sample_point: &Vec2) -> f32 {
    let mut density = 0.0;
    
    
    density
}

// fn property_gradient(sample_point: &Vec2) -> Vec2 {
//     
// }
