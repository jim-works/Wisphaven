use bevy::prelude::*;
use dashmap::DashMap;
use super::chunk::*;

#[derive(Resource)]
pub struct Level {
    pub chunks: DashMap<ChunkCoord, Chunk, ahash::RandomState>
}

impl Level {
    pub fn new() -> Level {
        Level {
            chunks: DashMap::with_hasher(ahash::RandomState::new())
        }
    }
    pub fn add_chunk(&mut self, key: ChunkCoord, chunk: Chunk) {
        self.chunks.insert(key,chunk);
    }
}