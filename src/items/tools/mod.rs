use bevy::prelude::*;
use bevy_rapier3d::prelude::{RapierContext, QueryFilter};
use serde::{Serialize, Deserialize};

use crate::world::{events::{BlockHitEvent, BlockDamageSetEvent}, Level, BlockId, LevelSystemSet, BlockCoord};

use super::SwingItemEvent;

pub mod abilities;

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(abilities::ToolAbilitiesPlugin)
            .register_type::<Tool>()
            .register_type::<ToolResistance>()
            .add_systems(Update, (on_swing, deal_block_damage).in_set(LevelSystemSet::Main))
        ;
    }
}

//denotes required power if attached to a block
//is used in `Tool` to give power of said tool
#[derive(Copy, Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub enum ToolResistance {
    Instant, //instantly broken
    Axe(u32),
    Pickaxe(u32),
    Shovel(u32),
}

impl Default for ToolResistance {
    fn default() -> Self {
        ToolResistance::Pickaxe(0)
    }
}

#[derive(Copy, Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Tool {
    pub axe: u32,
    pub pickaxe: u32,
    pub shovel: u32,
}

pub fn on_swing (
    mut reader: EventReader<SwingItemEvent>,
    mut writer: EventWriter<BlockHitEvent>,
    collision: Res<RapierContext>,
) {
    for SwingItemEvent(user, item, tf) in reader.iter() {
        if let Some((_, t)) = collision.cast_ray(tf.translation(), tf.forward(), 10.0, true, QueryFilter::new().exclude_collider(*user)) {
            let hit_pos = BlockCoord::from(tf.translation()+tf.forward()*(t+0.05)); //move into the block just a bit
            writer.send(BlockHitEvent { item: Some(item.id), user: Some(*user), block_position: hit_pos })
        }
    }
}

fn deal_block_damage (
    mut reader: EventReader<BlockHitEvent>,
    resistance_query: Query<&ToolResistance>,
    tool_query: Query<&Tool>,
    level: Res<Level>,
    id_query: Query<&BlockId>,
    mut writer: EventWriter<BlockDamageSetEvent>,
    mut commands: Commands
) {
    for BlockHitEvent { item, user, block_position } in reader.iter() {
        if let Some(block_hit) = level.get_block_entity(*block_position) {
            let resistance = resistance_query.get(block_hit).copied().unwrap_or_default();
            let tool = item.map(|i| tool_query.get(i).ok()).flatten().copied() //block was hit with a tool, so use that
                                .unwrap_or(user.map(|entity| tool_query.get(entity).ok()).flatten().copied() //entity that hit the block had tool power
                                .unwrap_or_default()); //...or nothing and use default tool
            level.damage_block(*block_position, calc_block_damage(resistance, tool), *item, &id_query, &mut writer, &mut commands);
        }
    }
}

pub fn calc_block_damage(resistance: ToolResistance, tool: Tool) -> f32 {
    const MAX_HITS_TO_BREAK: u32 = 5;
    match resistance {
        ToolResistance::Instant => 1.0,
        ToolResistance::Axe(required) => if required <= tool.axe {1.0/(MAX_HITS_TO_BREAK.saturating_sub(tool.axe.saturating_sub(required)) as f32).max(1.0)} else {0.0},
        ToolResistance::Pickaxe(required) => if required <= tool.pickaxe {1.0/(MAX_HITS_TO_BREAK.saturating_sub(tool.pickaxe.saturating_sub(required)) as f32).max(1.0)} else {0.0},
        ToolResistance::Shovel(required) => if required <= tool.shovel {1.0/(MAX_HITS_TO_BREAK.saturating_sub(tool.shovel.saturating_sub(required)) as f32).max(1.0)} else {0.0},
    }
}