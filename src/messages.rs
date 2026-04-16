use std::time::{Duration, Instant};

use bevy::math::Vec3;
use bevy::prelude::{default, Commands, Component, Justify, Query, Text2d, TextFont, TextLayout, Transform};

#[derive(Clone)]
pub struct MessageText {
    pub text: String,
    pub start_time: Instant,
    pub duration: Duration,
}

#[derive(Component)]
pub struct Messages {
    pub messages: Vec<MessageText>,
}

pub fn spawn_messages(commands: &mut Commands) {
    // Dynamic message text
    let mut messages = Messages { messages: vec![] };

    if cfg!(debug_assertions) {
        messages.messages.push(MessageText {
            text: "NOTE: the debug version looks like garbage.\nRun the release version for a better experience".into(),
            start_time: Instant::now(),
            duration: Duration::from_secs(3),
        });
    } else {
        messages.messages.push(MessageText {
            text: "Left-click to attract, right-click to repel\n\nPress ? for keyboard commands".into(),
            start_time: Instant::now(),
            duration: Duration::from_secs(2),
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
        messages,
    ));
}

pub fn display_messages(mut query: Query<(&mut Text2d, &mut Messages)>) {
    for (mut text, mut messages) in &mut query {
        // Remove expired messages
        messages.messages = messages
            .messages
            .iter()
            .filter_map(|message_text| {
                // if let Some(msg_text) = message_text.text.as_ref() {
                let duration = Instant::now().duration_since(message_text.start_time);
                if duration < message_text.duration {
                    Some(message_text.clone())
                } else {
                    None
                }
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
