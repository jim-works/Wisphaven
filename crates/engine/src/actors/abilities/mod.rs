use bevy::prelude::*;

pub mod dash;
pub mod stamina;

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((dash::DashPlugin, stamina::StaminaPlugin));
    }
}
