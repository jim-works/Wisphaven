use bevy::prelude::*;

use crate::util::LocalRepeatingTimer;

use super::{LevelSystemSet, Level, events::BlockDamageSetEvent};

pub struct BlocksPlugin;

pub mod tnt;

impl Plugin for BlocksPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(tnt::TNTPlugin)
            .add_systems(Update, heal_block_damages.in_set(LevelSystemSet::Main))
        ;
    }
}

fn heal_block_damages(
    mut timer: Local<LocalRepeatingTimer<100>>,
    time: Res<Time>,
    level: Res<Level>,
    mut writer: EventWriter<BlockDamageSetEvent>
) {
    //block heals over 20 seconds 1/(0.005/10)
    const HEAL_AMOUNT: f32 = 0.005;
    timer.tick(time.delta());
    if timer.just_finished() {
        level.heal_block_damages(HEAL_AMOUNT, &mut writer);
    }
}