// my modifications:
// - change from regex_lite to regex dependency
// - update `filter_out` and `filter_in` on `TextEditable` to be `Option<Regex>`
// - update `is_ignored` to take &Regex instead of &Vec<String> for filtering, and &str instead of String for key
// - update `is_ignored` call sites and the structs to use the new argument
// - remove state feature

// LICENSE COPIED FROM REPO (APACHE OR MIT)

// MIT License

// Copyright (c) 2024 Trung Do <dothanhtrung@pm.me>

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Copyright 2024 Trung Do <dothanhtrung@pm.me>

//! ### Plugin
//!
//! Add plugin `TextEditPlugin` to the app and define which states it will run in:
//!
//! ```rust
//! #[derive(Clone, Debug, Default, Eq, PartialEq, Hash, States)]
//! enum GameState {
//!     #[default]
//!     Menu,
//! }
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         // Add the plugin
//!         .add_plugins(TextEditPlugin::new(vec![GameState::Menu]))
//!         .run;
//! }
//! ```
//!
//! If you don't care to game state and want to always run input text, use `TextEditPluginNoState`:
//! ```rust
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     // Add the plugin
//!     .add_plugins(TextEditPluginNoState)
//!     .add_systems(Startup, setup)
//!     .run();
//! ```
//!
//! ### Component
//!
//! Insert component `TextEditable` into any text entity that needs to be editable:
//!
//! ```rust
//! commands.spawn((
//!     TextEditable::default(), // Mark text is editable
//!     Text::new("Input Text 1"),
//! ));
//! ```
//!
//! Only text that is focused by clicking gets keyboard input.
//!
//!
//! It is also possible to limit which characters are allowed to enter through `filter_in` and `filter_out` attribute. Regex is supported:
//! ```rust
//! commands.spawn((
//!     TextEditable {
//!         filter_in: vec!["[0-9]".into(), " ".into()], // Only allow number and space
//!         filter_out: vec!["5".into()],                // Ignore number 5
//!         ..default()
//!     },
//!     Text::new("Input Text 1"),
//! ));
//! ```

use bevy::app::{App, Plugin, Update};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::{in_state, States};
use bevy::prelude::{
    ButtonInput, Changed, Commands, Component, Deref, DerefMut, Entity, EventReader,
    IntoSystemConfigs, MouseButton, Query, Res, ResMut, Resource, Text, Time, Timer, TimerMode,
    With, Without,
};
use bevy::ui::Interaction;
use regex::Regex;

macro_rules! plugin_systems {
    ( ) => {
        (
            listen_changing_focus,
            focus_text_box,
            listen_keyboard_input,
            blink_cursor,
        )
            .chain()
    };
}

const DEFAULT_CURSOR: char = '|';
const BLINK_INTERVAL: f32 = 0.5;

/// Current position of cursor in the text.
#[derive(Component, Default)]
pub struct CursorPosition {
    pub pos: usize,
}

/// The text that will be displayed as cursor. Default is `|`.
#[derive(Resource, Deref, DerefMut)]
pub struct DisplayTextCursor(char);

/// Text cursor blink interval in millisecond.
#[derive(Resource, Deref, DerefMut)]
pub struct BlinkInterval(Timer);

/// The main plugin
#[derive(Default)]
pub struct TextEditPlugin<T>
where
    T: States,
{
    /// List of game state that this plugin will run in.
    pub states: Option<Vec<T>>,
}

impl<T> Plugin for TextEditPlugin<T>
where
    T: States,
{
    fn build(&self, app: &mut App) {
        app.insert_resource(DisplayTextCursor(DEFAULT_CURSOR))
            .insert_resource(BlinkInterval(Timer::from_seconds(
                BLINK_INTERVAL,
                TimerMode::Repeating,
            )));
        if let Some(states) = &self.states {
            for state in states {
                app.add_systems(Update, plugin_systems!().run_if(in_state(state.clone())));
            }
        } else {
            app.add_systems(Update, plugin_systems!());
        }
    }
}

impl<T> TextEditPlugin<T>
where
    T: States,
{
    pub fn new(states: Vec<T>) -> Self {
        Self {
            states: Some(states),
        }
    }
}

/// Use this if you don't care to state and want this plugin's systems run always.
#[derive(Default)]
pub struct TextEditPluginNoState;

impl Plugin for TextEditPluginNoState {
    fn build(&self, app: &mut App) {
        app.insert_resource(DisplayTextCursor(DEFAULT_CURSOR))
            .insert_resource(BlinkInterval(Timer::from_seconds(
                BLINK_INTERVAL,
                TimerMode::Repeating,
            )))
            .add_systems(Update, plugin_systems!());
    }
}

/// Mark a text entity is focused. Normally done by mouse click.
#[derive(Component)]
pub struct TextEditFocus;

/// Mark a text is editable.  
/// You can limit which characters are allowed to enter through `filter_in` and `filter_out` attribute. Regex is supported:
/// ```rust
/// commands.spawn((
///     TextEditable {
///         filter_in: vec!["[0-9]".into(), " ".into()], // Only allow number and space
///         filter_out: vec!["5".into()],                // Ignore number 5
///     },
///     Text::new("Input Text 1"),
/// ));
/// ```
#[derive(Component)]
#[require(Interaction)]
pub struct TextEditable {
    /// Character in this list won't be added to the text.
    pub filter_out: Option<Regex>,

    /// If not empty, only character in this list will be added to the text.
    pub filter_in: Option<Regex>,

    /// Maximum text length. Default is 254. 0 means unlimited.
    pub max_length: usize,

    /// Blink the text cursor.
    pub blink: bool,
}

impl Default for TextEditable {
    fn default() -> Self {
        Self {
            filter_out: Default::default(),
            filter_in: Default::default(),
            max_length: 254,
            blink: false,
        }
    }
}

fn unfocus_text_box(
    commands: &mut Commands,
    text_focus: &mut Query<(Entity, &CursorPosition, &mut Text), With<TextEditFocus>>,
    ignore_entity: Option<Entity>,
) {
    for (e, cursor, mut text) in text_focus.iter_mut() {
        if ignore_entity.is_none() || e != ignore_entity.unwrap() {
            commands.entity(e).remove::<TextEditFocus>();

            if text.len() > cursor.pos {
                text.remove(cursor.pos);
            }
            commands.entity(e).remove::<CursorPosition>();
            commands.entity(e).remove::<TextEditFocus>();
        }
    }
}

fn focus_text_box(
    mut commands: Commands,
    mut focused_texts: Query<(&mut Text, Entity), (With<TextEditFocus>, Without<CursorPosition>)>,
    display_cursor: Res<DisplayTextCursor>,
) {
    for (mut text, e) in focused_texts.iter_mut() {
        if !text.is_empty() {
            let pos = text.len();
            commands.entity(e).insert(CursorPosition { pos });
            text.push(**display_cursor);
        }
    }
}

pub fn listen_changing_focus(
    mut commands: Commands,
    input: Res<ButtonInput<MouseButton>>,
    mut text_interactions: Query<
        (&Interaction, Entity),
        (Changed<Interaction>, With<TextEditable>),
    >,
    other_interactions: Query<&Interaction, (Changed<Interaction>, Without<TextEditable>)>,
    mut focusing_texts: Query<(Entity, &CursorPosition, &mut Text), With<TextEditFocus>>,
) {
    let mut clicked_elsewhere = input.just_pressed(MouseButton::Left);
    for oth_itr in other_interactions.iter() {
        if *oth_itr == Interaction::Pressed {
            clicked_elsewhere = true;
        }
    }
    if text_interactions.is_empty() && clicked_elsewhere {
        unfocus_text_box(&mut commands, &mut focusing_texts, None);
        return;
    }

    for (interaction, e) in text_interactions.iter_mut() {
        if *interaction == Interaction::Pressed {
            let mut focusing_list = Vec::new();
            for (focusing_e, _, _) in focusing_texts.iter() {
                focusing_list.push(focusing_e);
            }

            unfocus_text_box(&mut commands, &mut focusing_texts, Some(e));
            if !focusing_list.contains(&e) {
                commands.entity(e).insert(TextEditFocus);
            }
        }
    }
}

fn listen_keyboard_input(
    mut events: EventReader<KeyboardInput>,
    mut edit_text: Query<(&mut Text, &mut CursorPosition, &TextEditable), With<TextEditFocus>>,
    display_cursor: Res<DisplayTextCursor>,
) {
    for event in events.read() {
        // Only trigger changes when the key is first pressed.
        if event.state == ButtonState::Released {
            continue;
        }

        for (mut text, mut cursor, texteditable) in edit_text.iter_mut() {
            let ignore_regex = texteditable.filter_out.as_ref();
            let allow_regex = texteditable.filter_in.as_ref();
            match &event.logical_key {
                Key::Space => {
                    if is_ignored(ignore_regex, allow_regex, " ")
                        || (texteditable.max_length > 0 && text.len() > texteditable.max_length)
                    {
                        continue;
                    }

                    text.insert(cursor.pos, ' ');
                    cursor.pos += 1;
                }
                Key::Backspace => {
                    if cursor.pos > 0 {
                        text.remove(cursor.pos - 1);
                        cursor.pos -= 1;
                    }
                }
                Key::Delete => {
                    if cursor.pos < text.len() - 1 {
                        text.remove(cursor.pos + 1);
                    }
                }
                Key::Character(character) => {
                    if is_ignored(ignore_regex, allow_regex, character)
                        || (texteditable.max_length > 0 && text.len() > texteditable.max_length)
                    {
                        continue;
                    }

                    text.insert_str(cursor.pos, character);
                    cursor.pos += character.len();
                }
                Key::ArrowLeft => {
                    if cursor.pos > 0 {
                        text.remove(cursor.pos);

                        cursor.pos -= 1;
                        text.insert(cursor.pos, **display_cursor);
                    }
                }
                Key::ArrowRight => {
                    if cursor.pos < text.len() - 1 {
                        text.remove(cursor.pos);

                        cursor.pos += 1;
                        text.insert(cursor.pos, **display_cursor);
                    }
                }
                Key::Home => {
                    text.remove(cursor.pos);
                    cursor.pos = 0;
                    text.insert(0, **display_cursor);
                }
                Key::End => {
                    text.remove(cursor.pos);
                    cursor.pos = text.len();
                    text.push(**display_cursor);
                }
                _ => continue,
            }
        }
    }
}

fn blink_cursor(
    time: Res<Time>,
    mut blink_interval: ResMut<BlinkInterval>,
    display_text_cursor: Res<DisplayTextCursor>,
    mut query: Query<(&mut Text, &CursorPosition, &TextEditable), With<TextEditFocus>>,
) {
    blink_interval.tick(time.delta());
    for (mut text, cursor_pos, text_editable) in query.iter_mut() {
        if text_editable.blink && blink_interval.just_finished() && text.len() > cursor_pos.pos {
            let current_cursor = text.as_bytes()[cursor_pos.pos] as char;
            let next_cursor = if current_cursor != **display_text_cursor {
                **display_text_cursor
            } else {
                ' '
            };
            text.replace_range(
                cursor_pos.pos..(cursor_pos.pos + 1),
                String::from(next_cursor).as_str(),
            );
        }
    }
}

fn is_ignored(ignored: Option<&Regex>, allowed: Option<&Regex>, key: &str) -> bool {
    if ignored.is_none_or(|re| re.is_match(key)) {
        return true;
    }

    if allowed.is_none_or(|re| re.is_match(key)) {
        return false;
    }

    true
}
