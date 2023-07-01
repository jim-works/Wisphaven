use bevy::prelude::*;
use serde::{Serialize, Deserialize};

use crate::world::{Level, BlockCoord, BlockType};

use super::UseItemEvent;

#[derive(Component)]
pub struct BlockItem(pub BlockType);

#[derive(Component)]
pub struct MegablockItem(pub BlockType, pub i32);

pub fn use_block_item(
    mut reader: EventReader<UseItemEvent>,
    block_query: Query<&BlockItem>,
    level: Res<Level>,
    mut commands: Commands,
) {
    for event in reader.iter() {
        if let Ok(block_item) = block_query.get(event.1.id) {
            if let Some(hit) = level.blockcast(event.2.translation(), event.2.forward()*10.0) {
                level.set_block(hit.block_pos+hit.normal, block_item.0, &mut commands);
            }
        }
    }
}

pub fn use_mega_block_item(
    mut reader: EventReader<UseItemEvent>,
    megablock_query: Query<&MegablockItem>,
    level: Res<Level>,
    mut commands: Commands,
) {
    for event in reader.iter() {
        if let Ok(block_item) = megablock_query.get(event.1.id) {
            let size = block_item.1;
            if let Some(hit) = level.blockcast(event.2.translation(), event.2.forward()*100.0) {
                let mut changes = Vec::with_capacity((size*size*size) as usize);
                for x in -size..size+1 {
                    for y in -size..size+1 {
                        for z in -size..size+1 {
                            changes.push((
                                hit.block_pos + BlockCoord::new(x, y, z),
                                block_item.0,
                            ));
                        }
                    }
                }
                level.batch_set_block(changes.into_iter(), &mut commands);
            }
        }
    }
}