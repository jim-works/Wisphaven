pub mod slither_spine;
pub mod spawning;

use bevy::prelude::*;

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((spawning::SpawningPlugin, slither_spine::SlitherSpinePlugin));
    }
}
