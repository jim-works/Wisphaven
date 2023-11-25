use bevy::{prelude::*, utils::HashSet};
use serde::{Deserialize, Serialize};

use crate::{
    util::Direction,
    world::{
        events::{BlockDamageSetEvent, BlockHitEvent, ChunkUpdatedEvent},
        BlockCoord, BlockId, Level, LevelSystemSet,
    },
};

use super::{calc_block_damage, Tool, ToolResistance};

pub struct ToolAbilitiesPlugin;

impl Plugin for ToolAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AxeAbility>()
            .register_type::<AxeAbilityTarget>()
            .register_type::<ShovelAbility>()
            .register_type::<ShovelAbilityTarget>()
            .add_systems(
                Update,
                (axe_ability_system, shovel_ability_system).in_set(LevelSystemSet::Main),
            );
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct AxeAbilityTarget;

#[derive(Copy, Clone, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct AxeAbility {
    pub max_blocks: usize,
    pub search_radius: i32,
    pub damage_mult: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ShovelAbilityTarget;

#[derive(Copy, Clone, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ShovelAbility {
    pub radius: usize,
    pub length: usize,
    pub damage_mult: f32,
}

fn axe_ability_system(
    level: Res<Level>,
    mut reader: EventReader<BlockHitEvent>,
    axe_ability_query: Query<(&Tool, &AxeAbility)>,
    resistance_query: Query<&ToolResistance>,
    id_query: Query<&BlockId>,
    target_query: Query<&AxeAbilityTarget>,
    mut commands: Commands,
    mut writer: EventWriter<BlockDamageSetEvent>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
) {
    for BlockHitEvent {
        item,
        user: _,
        block_position,
        hit_forward: _,
    } in reader.read()
    {
        if let Some(item) = item {
            if let Ok((
                tool,
                AxeAbility {
                    max_blocks,
                    search_radius,
                    damage_mult,
                },
            )) = axe_ability_query.get(*item)
            {
                if let Some(block) = level.get_block_entity(*block_position) {
                    if !target_query.contains(block) {
                        continue;
                    }
                    let damage = calc_block_damage(
                        resistance_query.get(block).copied().unwrap_or_default(),
                        *tool,
                    );
                    //trigger ability
                    do_axe_ability(
                        &level,
                        damage * damage_mult,
                        *block_position,
                        *max_blocks,
                        *search_radius,
                        &id_query,
                        &target_query,
                        *item,
                        &mut HashSet::new(),
                        &mut commands,
                        &mut writer,
                        &mut update_writer,
                    );
                }
            }
        }
    }
}

//returns blocks broken
//searches up first, expands in a square, then searches down
fn do_axe_ability(
    level: &Level,
    damage: f32,
    initial_pos: BlockCoord,
    max_blocks: usize,
    search_radius: i32,
    id_query: &Query<&BlockId>,
    target_query: &Query<&AxeAbilityTarget>,
    tool: Entity,
    hits: &mut HashSet<BlockCoord>,
    commands: &mut Commands,
    writer: &mut EventWriter<BlockDamageSetEvent>,
    update_writer: &mut EventWriter<ChunkUpdatedEvent>,
) -> usize {
    //ya know sometimes you just gotta ident a lot
    for square_radius in 0..search_radius + 1 {
        for y in 1..search_radius + 1 {
            for x in -square_radius..square_radius + 1 {
                for z in -square_radius..square_radius + 1 {
                    let pos = initial_pos + BlockCoord::new(x, y, z);
                    if let Some(block) = level.get_block_entity(pos) {
                        if target_query.contains(block) {
                            if hits.insert(pos) {
                                if pos != initial_pos {
                                    level.damage_block(
                                        pos,
                                        damage,
                                        Some(tool),
                                        id_query,
                                        writer,
                                        update_writer,
                                        commands,
                                    );
                                }
                                do_axe_ability(
                                    level,
                                    damage,
                                    pos,
                                    max_blocks,
                                    search_radius,
                                    id_query,
                                    target_query,
                                    tool,
                                    hits,
                                    commands,
                                    writer,
                                    update_writer,
                                );
                            }
                            if hits.len() >= max_blocks {
                                return max_blocks;
                            }
                        }
                    }
                }
            }
        }
    }
    //same thing but down
    for square_radius in 0..search_radius + 1 {
        for y in -search_radius..-1 {
            for x in -square_radius..square_radius + 1 {
                for z in -square_radius..square_radius + 1 {
                    let pos = initial_pos + BlockCoord::new(x, y, z);
                    if let Some(block) = level.get_block_entity(pos) {
                        if target_query.contains(block) {
                            if hits.insert(pos) {
                                if pos != initial_pos {
                                    level.damage_block(
                                        pos,
                                        damage,
                                        None,
                                        id_query,
                                        writer,
                                        update_writer,
                                        commands,
                                    );
                                }
                                do_axe_ability(
                                    level,
                                    damage,
                                    pos,
                                    max_blocks,
                                    search_radius,
                                    id_query,
                                    target_query,
                                    tool,
                                    hits,
                                    commands,
                                    writer,
                                    update_writer,
                                );
                            }
                            if hits.len() >= max_blocks {
                                return max_blocks;
                            }
                        }
                    }
                }
            }
        }
    }
    hits.len()
}

fn shovel_ability_system(
    level: Res<Level>,
    mut reader: EventReader<BlockHitEvent>,
    shovel_ability_query: Query<(&Tool, &ShovelAbility)>,
    resistance_query: Query<&ToolResistance>,
    id_query: Query<&BlockId>,
    target_query: Query<&ShovelAbilityTarget>,
    mut commands: Commands,
    mut writer: EventWriter<BlockDamageSetEvent>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
) {
    for BlockHitEvent {
        item,
        user: _,
        block_position,
        hit_forward,
    } in reader.read()
    {
        if let Some(item) = item {
            if let Ok((
                tool,
                ShovelAbility {
                    radius,
                    length,
                    damage_mult,
                },
            )) = shovel_ability_query.get(*item)
            {
                let direction = Direction::from(*hit_forward);
                let axis = BlockCoord::from(direction);
                info!("axis: {:?}", axis);
                for len in 0..(*length as i32) {
                    direction.for_each_in_plane(*radius as i32, |offset| {
                        let coord = axis * len + offset + *block_position;
                        if let Some(block) = level.get_block_entity(coord) {
                            if coord == *block_position || !target_query.contains(block) {
                                return;
                            }
                            let damage = calc_block_damage(
                                resistance_query.get(block).copied().unwrap_or_default(),
                                *tool,
                            ) * damage_mult;
                            level.damage_block(
                                coord,
                                damage,
                                Some(*item),
                                &id_query,
                                &mut writer,
                                &mut update_writer,
                                &mut commands,
                            );
                        }
                    });
                }
            }
        }
    }
}
