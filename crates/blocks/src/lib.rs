use bevy::prelude::*;

use util::LocalRepeatingTimer;

use interfaces::scheduling::LevelSystemSet;
use world::{events::BlockDamageSetEvent, level::Level};

pub struct BlocksPlugin;

pub mod fall;
pub mod tnt;

impl Plugin for BlocksPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((tnt::TNTPlugin, fall::FallPlugin))
            .add_systems(Update, heal_block_damages.in_set(LevelSystemSet::Main));
    }
}

const HEAL_CHECK_INTERVAL_MS: u64 = 100;

fn heal_block_damages(
    mut timer: Local<LocalRepeatingTimer<{ HEAL_CHECK_INTERVAL_MS }>>,
    time: Res<Time>,
    level: Res<Level>,
    mut writer: EventWriter<BlockDamageSetEvent>,
) {
    timer.tick(time.delta());
    if timer.just_finished() {
        level.heal_block_damages(timer.duration().as_secs_f32(), &mut writer);
    }
}
