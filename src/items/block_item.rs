use bevy::prelude::*;

use crate::world::{Level, BlockCoord, BlockResources, BlockName, BlockId, events::ChunkUpdatedEvent};

use super::UseItemEvent;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct BlockItem(pub BlockName);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct MegaBlockItem(pub BlockName, pub i32);

pub fn use_block_item(
    mut reader: EventReader<UseItemEvent>,
    block_query: Query<&BlockItem>,
    level: Res<Level>,
    resources: Res<BlockResources>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    id_query: Query<&BlockId>,
    mut commands: Commands,
) {
    for UseItemEvent { user: _, inventory_slot: _, stack, tf } in reader.iter() {
        if let Ok(block_item) = block_query.get(stack.id) {
            if let Some(hit) = level.blockcast(tf.translation(), tf.forward()*10.0) {
                let id = resources.registry.get_id(&block_item.0);
                level.set_block(hit.block_pos+hit.normal, id, &resources.registry, &id_query, &mut update_writer, &mut commands);
            }
        }
    }
}

pub fn use_mega_block_item(
    mut reader: EventReader<UseItemEvent>,
    megablock_query: Query<&MegaBlockItem>,
    level: Res<Level>,
    resources: Res<BlockResources>,
    id_query: Query<&BlockId>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    mut commands: Commands,
) {
    for UseItemEvent { user: _, inventory_slot: _, stack, tf } in reader.iter() {
        if let Ok(block_item) = megablock_query.get(stack.id) {
            let id = resources.registry.get_id(&block_item.0);
            let size = block_item.1;
            if let Some(hit) = level.blockcast(tf.translation(), tf.forward()*100.0) {
                let mut changes = Vec::with_capacity((size*size*size) as usize);
                for x in -size..size+1 {
                    for y in -size..size+1 {
                        for z in -size..size+1 {
                            changes.push((
                                hit.block_pos + BlockCoord::new(x, y, z),
                                id,
                            ));
                        }
                    }
                }
                level.batch_set_block(changes.into_iter(), &resources.registry, &id_query, &mut update_writer, &mut commands);
            }
        }
    }
}