pub mod damaged_block;

use bevy::prelude::*;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(damaged_block::DamagedBlockPlugin);
    }
}