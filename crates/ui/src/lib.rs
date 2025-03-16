use bevy::prelude::*;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ui_core::UICorePlugin,
            ui_state::UIStatePlugin,
            ui_inventory::UIInventoryPlugin,
            ui_combat::PlayerStatsUiPlugin,
            ui_crafting::UICraftingPlugin,
            ui_menu::MainMenuPlugin,
            ui_crosshair::CrosshairPlugin,
            ui_waves::WavesPlugin,
            ui_game_over::UIGameOverPlugin,
        ));
    }
}
