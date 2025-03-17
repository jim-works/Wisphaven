pub mod block_actors;
pub mod coin;
mod eye_balloon;
pub mod skeleton_pirate;
pub mod slither_spine;
pub mod spawning;
pub mod spike_ball;
mod util;

use bevy::prelude::*;

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            spawning::SpawningPlugin,
            slither_spine::SlitherSpinePlugin,
            coin::CoinPlugin,
            skeleton_pirate::SkeletonPiratePlugin,
            spike_ball::SpikeBallPlugin,
            eye_balloon::EyeBalloonPlugin,
            util::ActorUtilPlugin,
            block_actors::BlockActorPlugin,
        ));
    }
}
