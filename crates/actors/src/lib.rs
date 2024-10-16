pub mod coin;
pub mod skeleton_pirate;
pub mod slither_spine;
pub mod spawning;
pub mod spike_ball;
mod wisp;

use bevy::prelude::*;

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            spawning::SpawningPlugin,
            slither_spine::SlitherSpinePlugin,
            coin::CoinPlugin,
            skeleton_pirate::SkeletonPiratePlugin,
            wisp::WispPlugin,
            spike_ball::SpikeBallPlugin,
        ));
    }
}
