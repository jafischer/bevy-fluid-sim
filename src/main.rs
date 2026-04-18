mod args;
mod components;
mod events;
mod keyboard;
mod messages;
mod sim_impl;
mod sim_settings;
mod sim_struct;
mod update;

use bevy::color::palettes::css::GOLD;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use bevy_embedded_assets::EmbeddedAssetPlugin;
use clap::Parser;

use crate::args::Args;
use crate::components::*;
use crate::events::{handle_keypress, handle_mouse_clicks, on_resize};
use crate::messages::{MessageText, display_messages, spawn_messages};
use crate::sim_struct::Simulation;
use crate::update::{draw_debug_info, update_fps, update_particles};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let (width, height) = args.win_size()?;

    // Create and run the Bevy App.
    App::new()
        // Background color
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 0.05)))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(width, height),
                    ..default()
                }),
                ..default()
            }),
            // The EmbeddedAssetPlugin loads all files under assets/ (at compile time) and makes them available via
            // asset_server.load("embedded://filename").
            EmbeddedAssetPlugin::default(),
        ))
        // Add our startup function, setup().
        .add_systems(Startup, setup)
        // Add the functions that will be called once per update.
        .add_systems(
            Update,
            (
                update_particles,
                draw_debug_info,
                handle_keypress,
                handle_mouse_clicks,
                on_resize,
                update_fps,
                display_messages,
            ),
        )
        .insert_resource(ArgsResource(args))
        .run();

    Ok(())
}

fn setup(mut commands: Commands, window: Single<&Window>, asset_server: Res<AssetServer>, args: Res<ArgsResource>) {
    commands.spawn(Camera2d);

    // Create the simulation and add it to ECS.
    // Note: the simulation isn't well-integrated into Bevy ECS at all. Perhaps I will try, at some point,
    // to move the many buffers inside the Simulation struct (e.g. positions, velocities, densities, and so on)
    // into ECS, but it was easier to just stick them inside Simulation while developing.
    // It would be interesting to see what, if any, impact moving them to ECS has on performance.
    let mut sim = Simulation::new(window.width(), window.height(), &args.0);

    sim.spawn_particles(&mut commands);
    commands.spawn(sim);

    // FPS display.
    commands.spawn((
        Text::default(),
        (
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(GOLD.into()),
        ),
        FpsText,
    ));

    // Add a text display of the number of particles.
    commands.spawn((
        Text::new(format!("{} particles", args.0.num_particles)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Right),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(5.0),
            ..default()
        },
    ));

    // Spawn the popup message component.
    spawn_messages(&mut commands);

    // Keyboard commands component
    commands.spawn(KeyboardCommands::create());

    // Load the image for the sprites.
    commands.spawn(SpriteImage {
        handle: asset_server.load("embedded://blurred-circle-pow-2.0.png"),
    });
}
