pub mod styles;
pub mod inventory;
pub mod healthbar;
pub mod crosshair;
pub mod state;
pub mod debug;

use bevy::prelude::*;
use bevy::window::CursorGrabMode;


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
        ;
    }
}

fn capture_mouse(
    mut window_query: Query<&mut Window>,
) {
    let mut window = window_query.get_single_mut().unwrap();
    window.set_cursor_grab_mode(CursorGrabMode::Locked);
}

fn release_mouse (
    mut window_query: Query<&mut Window>,
) {

}