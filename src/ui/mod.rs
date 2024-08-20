pub mod crosshair;
pub mod debug;
pub mod healthbar;
pub mod inventory;
pub mod player_stats;
pub mod state;
pub mod styles;
pub mod waves;

use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use leafwing_input_manager::action_state::ActionState;

use crate::actors::LocalPlayer;
use crate::controllers::Action;
use crate::world::LevelSystemSet;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<state::UIState>()
            .add_systems(Startup, styles::init)
            .add_plugins((
                inventory::InventoryPlugin,
                crosshair::CrosshairPlugin,
                healthbar::HealthbarPlugin,
                player_stats::PlayerStatsUiPlugin,
                waves::WavesPlugin,
            ))
            .add_plugins(debug::DebugUIPlugin)
            .add_systems(OnEnter(state::UIState::Default), capture_mouse)
            .add_systems(OnEnter(state::UIState::Inventory), release_mouse)
            .add_systems(
                Update,
                (state::toggle_hidden, state::toggle_debug).in_set(LevelSystemSet::Main),
            )
            .add_systems(Update, toggle_fullscreen)
            .insert_resource(UiScale(2.0));
    }
}

pub fn world_mouse_active(state: &state::UIState) -> bool {
    match state {
        state::UIState::Hidden => true,
        state::UIState::Default => true,
        state::UIState::Inventory => false,
    }
}

fn capture_mouse(mut window_query: Query<&mut Window>) {
    let mut window = window_query.get_single_mut().unwrap();
    window.cursor.grab_mode = CursorGrabMode::Locked;
    window.cursor.visible = false;
}

fn release_mouse(mut window_query: Query<&mut Window>) {
    let mut window = window_query.get_single_mut().unwrap();
    window.cursor.grab_mode = CursorGrabMode::None;
    window.cursor.visible = true;
}

fn toggle_fullscreen(
    mut window_query: Query<&mut Window>,
    input_query: Query<&ActionState<Action>, With<LocalPlayer>>,
) {
    if let Ok(input) = input_query.get_single() {
        if input.just_pressed(Action::ToggleFullscreen) {
            let mut window = window_query.get_single_mut().unwrap();
            window.mode = match window.mode {
                bevy::window::WindowMode::Windowed => {
                    bevy::window::WindowMode::BorderlessFullscreen
                }
                _ => bevy::window::WindowMode::Windowed,
            };
        }
    }
}
