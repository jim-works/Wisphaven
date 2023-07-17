use bevy::prelude::*;

use crate::util::LocalRepeatingTimer;

use super::{LevelSystemSet, Level};

pub struct BlocksPlugin;

pub mod tnt;

impl Plugin for BlocksPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(tnt::TNTPlugin)
            .add_system(heal_block_damages.in_set(LevelSystemSet::Main))
        ;
    }
}

fn heal_block_damages(
    mut timer: Local<LocalRepeatingTimer<100>>,
    time: Res<Time>,
    level: Res<Level>
) {
    const HEAL_AMOUNT: f32 = 0.01;
    timer.tick(time.delta());
    if timer.just_finished() {
        level.heal_block_damages(HEAL_AMOUNT);
    }
}