use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const NUM_PARTICLES: u32 = 2000;
const GRAVITY: f32 = 9.8;
const PARTICLE_MASS: f32 = 1.0;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>
) {
    commands.spawn(Camera2dBundle::default());
    let window = window_query.get_single().unwrap();
    let max_h = window.height() * 0.67;

    let (size, cols, _rows) = subdivide_into_squares(
        window.width(),
        max_h,
        NUM_PARTICLES
    );

    let particle_size = size * 0.8;
    let scale = Vec3::splat(particle_size / 128.0);
    let shift = Vec3 {
        x: -window.width() / 2.0 + size / 2.0,
        y: -(window.height() - max_h) / 2.0 + size / 2.0,
        z: 0.0
    };
    
    // Make it even
    let num_particles = NUM_PARTICLES - (NUM_PARTICLES % cols);

    let texture = asset_server.load("grey.png");
    for i in 0..num_particles {
        let x = (i % cols) as f32 * size;
        let y = (i / cols) as f32 * size;
        commands.spawn((
            SpriteBundle {
                texture: texture.clone(),
                transform: Transform {
                    translation: Vec3 { x, y, z: 0.0 } + shift,
                    scale,
                    ..default()
                },
                ..default()
            },
        ));
    }
}

fn update() {}

fn subdivide_into_squares(w: f32, h: f32, n: u32) -> (f32, u32, u32) {
    // Step 1: Calculate the target area of each square
    let target_area = (w * h) / n as f32;

    // Step 2: Calculate the side length of each square
    let side_length = target_area.sqrt();

    // Step 3: Calculate the number of columns and rows
    let columns = (w / side_length).ceil() as u32;
    let rows = (n as f32 / columns as f32).ceil() as u32;

    // Step 4: Adjust the final side length to fit evenly
    let final_side_length = f32::min(w / columns as f32, h / rows as f32);

    (final_side_length, columns, rows)
}
