pub mod styles;
pub mod inventory;
pub mod healthbar;
pub mod crosshair;
pub mod state;
pub mod debug;

use bevy::prelude::*;
use bevy::window::CursorGrabMode;

use crate::world::LevelSystemSet;


pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_state::<state::UIState>()
            .add_systems(Startup, styles::init)
            .add_plugins(inventory::InventoryPlugin)
            .add_plugins(crosshair::CrosshairPlugin)
            .add_plugins(healthbar::HealthbarPlugin)
            .add_plugins(debug::DebugUIPlugin)
            .add_systems(OnEnter(state::UIState::Default), capture_mouse)
            .add_systems(OnEnter(state::UIState::Inventory), release_mouse)
            .add_systems(Update, (state::toggle_hidden, state::toggle_debug).in_set(LevelSystemSet::Main))
        ;
    }
}

pub fn world_mouse_active(state: &state::UIState) -> bool {
    match state {
        state::UIState::Hidden => true,
        state::UIState::Default => true,
        state::UIState::Inventory => false,
    }
}

fn capture_mouse(
    mut window_query: Query<&mut Window>,
) {
    let mut window = window_query.get_single_mut().unwrap();
    window.cursor.grab_mode = CursorGrabMode::Locked;
    window.cursor.visible = false;
}

fn release_mouse (
    mut window_query: Query<&mut Window>,
) {
    let mut window = window_query.get_single_mut().unwrap();
    window.cursor.grab_mode = CursorGrabMode::None;
    window.cursor.visible = true;
}