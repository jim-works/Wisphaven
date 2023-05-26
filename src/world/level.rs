use bevy::prelude::*;
use dashmap::DashMap;
use super::{chunk::*, octree::Octree};

#[derive(Resource)]
pub struct Level {
    pub chunks: DashMap<ChunkCoord, ChunkType, ahash::RandomState>,
    pub octree: Octree
}

impl Level {
    pub fn new() -> Level {
        Level {
            chunks: DashMap::with_hasher(ahash::RandomState::new()),
            octree: Octree::new()
        }
    }
    pub fn add_chunk(&mut self, key: ChunkCoord, chunk: ChunkType) {
        self.chunks.insert(key,chunk);
    }
}