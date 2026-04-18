use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::args::Args;
use crate::keyboard::KeyboardCommand;
use crate::messages::MessageText;

#[derive(Resource)]
pub struct ArgsResource(pub Args);

#[derive(Component)]
pub struct FpsText;

/// Contains the collection of keyboard commands.
#[derive(Component)]
pub struct KeyboardCommands {
    pub commands: BTreeMap<KeyCode, KeyboardCommand>,
}

/// This component displays messages in the middle of the main window. Notifications have a duration, and multiple messages
/// will scroll up.
#[derive(Component)]
pub struct Notifications {
    pub messages: Vec<MessageText>,
}

#[derive(Component, Clone, Debug, Default)]
pub struct Particle {
    pub id: usize,
    pub watched: bool,
}

#[derive(Component)]
pub struct SpriteImage {
    pub handle: Handle<Image>,
}
