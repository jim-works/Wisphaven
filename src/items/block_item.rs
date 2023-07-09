use bevy::prelude::*;

use crate::world::{Level, BlockCoord, BlockResources, BlockName, BlockId};

use super::UseItemEvent;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct BlockItem(pub BlockName);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct MegablockItem(pub BlockName, pub i32);

pub fn use_block_item(
    mut reader: EventReader<UseItemEvent>,
    block_query: Query<&BlockItem>,
    level: Res<Level>,
    resources: Res<BlockResources>,
    id_query: Query<&BlockId>,
    mut commands: Commands,
) {
    for event in reader.iter() {
        if let Ok(block_item) = block_query.get(event.1.id) {
            if let Some(hit) = level.blockcast(event.2.translation(), event.2.forward()*10.0) {
                let id = resources.registry.get_id(&block_item.0);
                level.set_block(hit.block_pos+hit.normal, id, resources.registry.as_ref(), &id_query, &mut commands);
            }
        }
    }
}

pub fn use_mega_block_item(
    mut reader: EventReader<UseItemEvent>,
    megablock_query: Query<&MegablockItem>,
    level: Res<Level>,
    resources: Res<BlockResources>,
    id_query: Query<&BlockId>,
    mut commands: Commands,
) {
    for event in reader.iter() {
        if let Ok(block_item) = megablock_query.get(event.1.id) {
            let id = resources.registry.get_id(&block_item.0);
            let size = block_item.1;
            if let Some(hit) = level.blockcast(event.2.translation(), event.2.forward()*100.0) {
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
                level.batch_set_block(changes.into_iter(), resources.registry.as_ref(), &id_query, &mut commands);
            }
        }
    }
}