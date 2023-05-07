use bevy::prelude::*;
use crate::worldgen::worldgen::ChunkGeneratedEvent;

use super::level::Level;

pub fn on_chunk_generated(
    mut event: EventReader<ChunkGeneratedEvent>,
    mut level: ResMut<Level>
) {
    for c in event.iter() {
        level.add_chunk(c.chunk.position, c.chunk)
    }
}