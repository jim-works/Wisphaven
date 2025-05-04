#![feature(let_chains)]
use bevy::{prelude::*, window::CursorGrabMode};
use bevy_simple_text_input::TextInputInactive;
use engine::{actors::LocalPlayer, controllers::Action};
use interfaces::scheduling::{GameState, LevelSystemSet};
use leafwing_input_manager::prelude::ActionState;
use ui_core::{BORDER_COLOR_ACTIVE, BORDER_COLOR_INACTIVE};

pub struct UIStatePlugin;

impl Plugin for UIStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<UIState>()
            .init_state::<InputFocused>()
            .add_systems(OnEnter(GameState::Game), on_load)
            .add_systems(
                Update,
                (
                    toggle_hidden.in_set(LevelSystemSet::Main),
                    toggle_fullscreen,
                    update_focus,
                ),
            )
            .add_systems(OnEnter(GameState::Game), (on_load, capture_mouse))
            .add_systems(OnEnter(UIState::Default), capture_mouse)
            .add_systems(OnEnter(UIState::Inventory), release_mouse)
            .add_systems(OnExit(GameState::Game), release_mouse);
    }
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UIState {
    #[default]
    Hidden,
    Default,
    Inventory,
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InputFocused {
    #[default]
    Unfocused,
    Focused,
}

#[derive(Component)]
struct FocusedEntity;

pub fn toggle_hidden(
    mut next_state: ResMut<NextState<UIState>>,
    curr_state: Res<State<UIState>>,
    action: Res<ActionState<Action>>,
) {
    if action.just_pressed(&Action::ToggleUIHidden) {
        match curr_state.get() {
            UIState::Hidden => next_state.set(UIState::Default),
            _ => next_state.set(UIState::Hidden),
        }
    }
}

pub fn on_load(mut next_state: ResMut<NextState<UIState>>) {
    next_state.set(UIState::Default);
}

fn toggle_fullscreen(mut window_query: Query<&mut Window>, action: Res<ActionState<Action>>) {
    if action.just_pressed(&Action::ToggleFullscreen) {
        let mut window = window_query.get_single_mut().unwrap();
        window.mode = match window.mode {
            bevy::window::WindowMode::Windowed => {
                bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            }
            _ => bevy::window::WindowMode::Windowed,
        };
    }
}

pub fn world_mouse_active(state: &UIState) -> bool {
    match state {
        UIState::Hidden => true,
        UIState::Default => true,
        UIState::Inventory => false,
    }
}

fn capture_mouse(mut window_query: Query<&mut Window>) {
    let mut window = window_query.get_single_mut().unwrap();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

fn release_mouse(mut window_query: Query<&mut Window>) {
    let mut window = window_query.get_single_mut().unwrap();
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}

// bevy 0.16 - update using example here https://github.com/rparrett/bevy_simple_text_input/commits/main/examples/focus.rs
fn update_focus(
    query: Query<(Entity, &Interaction), (With<TextInputInactive>, Changed<Interaction>)>,
    text_input_query: Query<(Entity, &TextInputInactive, &BorderColor)>,
    focused: Query<Entity, With<FocusedEntity>>,
    curr_state: Res<State<InputFocused>>,
    mut next_focused: ResMut<NextState<InputFocused>>,
    mut commands: Commands,
) {
    //clear focus if entity despawns or loses component
    if matches!(curr_state.get(), InputFocused::Focused) {
        if let Ok(focused_entity) = focused.get_single()
            && !text_input_query.contains(focused_entity)
        {
            next_focused.set(InputFocused::Unfocused);
        }
        if focused.is_empty() {
            next_focused.set(InputFocused::Unfocused);
        }
    }
    //set focus if text box clicked
    for (interaction_entity, interaction) in &query {
        if *interaction == Interaction::Pressed && text_input_query.contains(interaction_entity) {
            commands.entity(interaction_entity).insert(FocusedEntity);
            next_focused.set(InputFocused::Focused);
        }
    }
    match next_focused.as_ref() {
        NextState::Unchanged => {}
        NextState::Pending(InputFocused::Unfocused) => commands.run_system_cached(on_unfocus),
        NextState::Pending(InputFocused::Focused) => commands.run_system_cached(on_focus),
    }
}

fn on_focus(
    action_player_query: Query<Entity, (With<LocalPlayer>, With<ActionState<Action>>)>,
    focused: Query<Entity, With<FocusedEntity>>,
    mut text_input_query: Query<(Entity, &mut TextInputInactive, &mut BorderColor)>,
    mut commands: Commands,
) {
    info!("on focused ran");
    let Ok(focused_entity) = focused.get_single() else {
        error!("somehow not focused in on_focus");
        return;
    };
    //update ui
    for (entity, mut inactive, mut border_color) in &mut text_input_query {
        if entity == focused_entity {
            inactive.0 = false;
            *border_color = BORDER_COLOR_ACTIVE;
        } else {
            inactive.0 = true;
            *border_color = BORDER_COLOR_ACTIVE;
        }
    }
    if let Ok(player_entity) = action_player_query.get_single() {
        // disable player controls while typing
        commands
            .entity(player_entity)
            .remove::<ActionState<Action>>();
    }
}

fn on_unfocus(
    no_action_player_query: Query<Entity, (With<LocalPlayer>, Without<ActionState<Action>>)>,
    mut text_input_query: Query<(&mut TextInputInactive, &mut BorderColor)>,
    mut commands: Commands,
) {
    info!("on unfocus ran");
    //update ui
    for (mut inactive, mut border_color) in &mut text_input_query {
        inactive.0 = true;
        *border_color = BORDER_COLOR_INACTIVE;
    }
    if let Ok(player_entity) = no_action_player_query.get_single() {
        // enable player controls again
        commands
            .entity(player_entity)
            .insert(ActionState::<Action>::default());
    }
}
