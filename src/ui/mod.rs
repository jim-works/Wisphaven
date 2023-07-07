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
            .add_startup_system(styles::init)
            .add_plugin(inventory::InventoryPlugin)
            .add_plugin(crosshair::CrosshairPlugin)
            .add_plugin(healthbar::HealthbarPlugin)
            .add_plugin(debug::DebugUIPlugin)
            .add_system(capture_mouse.in_schedule(OnEnter(state::UIState::Default)))
            .add_system(release_mouse.in_schedule(OnEnter(state::UIState::Inventory)))
            .add_system(state::toggle_hidden.in_set(LevelSystemSet::Main))
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