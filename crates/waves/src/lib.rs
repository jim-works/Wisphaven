use bevy::prelude::*;

pub mod waves;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(waves::WavesPlugin);
    }
}
