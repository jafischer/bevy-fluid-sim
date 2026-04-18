use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::sprite::Text2dShadow;

use crate::components::Notifications;

#[derive(Clone)]
pub struct MessageText {
    pub text: String,
    pub start_time: Instant,
    pub duration: Duration,
}

pub fn spawn_messages(commands: &mut Commands) {
    // Dynamic message text
    let mut messages = Notifications { messages: vec![] };

    // Add the startup message.
    if cfg!(debug_assertions) {
        messages.messages.push(MessageText {
            text: "   NOTE: the debug version looks like garbage.\nRun the release version for a better experience.\n\n         Click the mouse to continue...".into(),
            start_time: Instant::now(),
            duration: Duration::MAX,
        });
    } else {
        messages.messages.push(MessageText {
            text: "Left/right-click & drag to make the fluid dance!\n\n       Press ? for keyboard commands.\n\n       Click the mouse to continue...".into(),
            start_time: Instant::now(),
            duration: Duration::MAX,
        });
    }

    commands.spawn((
        Text2d::default(),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Left),
        Transform::from_translation(Vec3::new(0.0, 2.0, 1.0)),
        Text2dShadow {
            offset: Vec2::new(2., -2.),
            color: Color::BLACK,
        },
        messages,
    ));
}

pub fn display_messages(mut query: Query<(&mut Text2d, &mut Notifications)>) {
    for (mut text, mut messages) in &mut query {
        // Remove expired messages
        messages.messages = messages
            .messages
            .iter()
            .filter_map(|message_text| {
                let duration = Instant::now().duration_since(message_text.start_time);
                if duration < message_text.duration { Some(message_text.clone()) } else { None }
            })
            .collect();
        **text = messages
            .messages
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<&str>>()
            .join("\n");
    }
}
