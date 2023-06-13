use bevy::prelude::*;

use crate::world::{Level, BlockCoord};

use super::{UseItemEvent, ItemType};

pub fn use_block_item(
    mut reader: EventReader<UseItemEvent>,
    level: Res<Level>,
    mut commands: Commands,
) {
    for event in reader.iter() {
        if let ItemType::Block(block_type) = event.1 {
            if let Some(hit) = level.blockcast(event.2.translation(), event.2.forward()*10.0) {
                level.set_block(hit.block_pos+hit.normal, block_type, &mut commands);
            }
        }
    }
}

pub fn use_mega_block_item(
    mut reader: EventReader<UseItemEvent>,
    level: Res<Level>,
    mut commands: Commands,
) {
    for event in reader.iter() {
        if let ItemType::MegaBlock(block_type, size) = event.1 {
            if let Some(hit) = level.blockcast(event.2.translation(), event.2.forward()*100.0) {
                let mut changes = Vec::with_capacity((size*size*size) as usize);
                for x in -size..size+1 {
                    for y in -size..size+1 {
                        for z in -size..size+1 {
                            changes.push((
                                hit.block_pos + BlockCoord::new(x, y, z),
                                block_type,
                            ));
                        }
                    }
                }
                level.batch_set_block(changes.into_iter(), &mut commands);
            }
        }
    }
}