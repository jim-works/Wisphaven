use bevy::{prelude::*, window::CursorGrabMode};
use interfaces::scheduling::{GameState, LevelSystemSet};
use leafwing_input_manager::prelude::ActionState;

use engine::controllers::Action;

pub struct UIStatePlugin;

impl Plugin for UIStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<UIState>()
            .add_systems(OnEnter(GameState::Game), on_load)
            .add_systems(
                Update,
                (
                    toggle_hidden.in_set(LevelSystemSet::Main),
                    toggle_fullscreen,
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
