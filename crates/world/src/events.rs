use super::{
    chunk::ChunkCoord, level::Level, BlockCoord, BlockDamage, BlockId, BlockResources, BlockType,
    LevelSystemSet,
};
use bevy::prelude::*;
use interfaces::components::*;

pub struct WorldEventsPlugin;

impl Plugin for WorldEventsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ExplosionEvent>()
            .add_event::<BlockUsedEvent>()
            .add_event::<DealBlockDamageEvent>()
            .add_event::<BlockDamageSetEvent>()
            .add_event::<BlockHitEvent>()
            .add_event::<ChunkUpdatedEvent>()
            .add_event::<BlockBrokenEvent>()
            .add_systems(
                FixedUpdate,
                (process_explosions, process_block_damages)
                    .chain()
                    .in_set(LevelSystemSet::Tick),
            );
    }
}

#[derive(Event)]
pub struct BlockUsedEvent {
    pub block_position: BlockCoord,
    pub user: Entity,
    pub use_forward: Dir3,
    pub block_used: Entity,
}

#[derive(Event)]
//triggered when block gets punched
pub struct BlockHitEvent {
    pub item: Option<Entity>,
    pub user: Option<Entity>,
    pub hit_forward: Dir3,
    pub block_position: BlockCoord,
}

#[derive(Event, Clone, Copy)]
pub struct DealBlockDamageEvent {
    pub block_position: BlockCoord,
    // ranges from 0-1. 1 damage = destroyed block
    pub damage: f32,
    // the item, block, or entity that damaged the block.
    // the player's pickaxe, not the player
    pub damager: Option<Entity>,
}

#[derive(Event)]
pub struct BlockDamageSetEvent {
    pub block_position: BlockCoord,
    pub damage: BlockDamage,
    pub damager: Option<Entity>,
}

#[derive(Event)]
pub struct ExplosionEvent {
    pub radius: f32,
    pub origin: BlockCoord,
}

//triggered when a chunk is spawned in or a block is changed
#[derive(Event)]
pub struct ChunkUpdatedEvent {
    pub coord: ChunkCoord,
}

// not sent for all block breaks -- some (batch updates) don't trigger events
#[derive(Event)]
pub struct BlockBrokenEvent {
    pub coord: BlockCoord,
    pub block: Entity,
    pub broken_by: Option<Entity>,
}

fn process_explosions(
    mut reader: EventReader<ExplosionEvent>,
    level: Res<Level>,
    mut commands: Commands,
    id_query: Query<&BlockId>,
    resources: Res<BlockResources>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
) {
    for event in reader.read() {
        let size = event.radius.ceil() as i32;
        let mut changes = Vec::with_capacity((size * size * size) as usize);
        for x in -size..size + 1 {
            for y in -size..size + 1 {
                for z in -size..size + 1 {
                    if x * x + y * y + z * z <= size * size {
                        changes.push((event.origin + BlockCoord::new(x, y, z), BlockId(Id::Empty)));
                    }
                }
            }
        }
        level.batch_set_block(
            changes.into_iter(),
            &resources.registry,
            &id_query,
            &mut update_writer,
            &mut commands,
        );
    }
}

fn process_block_damages(
    mut damage_reader: EventReader<DealBlockDamageEvent>,
    id_query: Query<&BlockId>,
    level: Res<Level>,
    mut damage_writer: EventWriter<BlockDamageSetEvent>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    mut break_writer: EventWriter<BlockBrokenEvent>,
    mut commands: Commands,
) {
    for DealBlockDamageEvent {
        block_position,
        damage,
        damager,
    } in damage_reader.read().copied()
    {
        let mut remove_block = false;
        let mut remove_damage = false;
        let Some(entity) = level.get_block_entity(block_position) else {
            continue;
        };
        if damage == 0.0 {
            continue; //can't damage an empty block, or we did literally no damage
        }
        match level.block_damages.get_mut(&block_position) {
            Some(mut dam) => {
                let mut prev_damage = dam.value().with_time_reset();
                prev_damage.damage = (prev_damage.damage + damage).clamp(0.0, 1.0);
                *dam.value_mut() = prev_damage;
                if prev_damage.damage == 1.0 {
                    //total damage = 1, remove the block
                    remove_block = true;
                } else if prev_damage.damage == 0.0 {
                    //no more damage, so remove the damage value
                    remove_damage = true;
                }
                damage_writer.send(BlockDamageSetEvent {
                    block_position,
                    damage: prev_damage,
                    damager,
                });
            }
            None => {
                if damage < 1.0 {
                    let block_damage = BlockDamage::new(damage);
                    level.block_damages.insert(block_position, block_damage);
                    damage_writer.send(BlockDamageSetEvent {
                        block_position,
                        damage: block_damage,
                        damager,
                    });
                } else {
                    remove_block = true;
                    damage_writer.send(BlockDamageSetEvent {
                        block_position,
                        damage: BlockDamage::new(1.0),
                        damager,
                    });
                }
            }
        }
        if remove_block || remove_damage {
            level.block_damages.remove(&block_position);
        }
        if remove_block {
            break_writer.send(BlockBrokenEvent {
                coord: block_position,
                block: entity,
                broken_by: damager,
            });
            level.set_block_entity(
                block_position,
                BlockType::Empty,
                &id_query,
                &mut update_writer,
                &mut commands,
            );
        }
    }
}
