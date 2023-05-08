use bevy::prelude::*;
use dashmap::DashMap;
use super::chunk::*;

#[derive(Resource)]
pub struct Level {
    pub chunks: DashMap<ChunkCoord, ChunkType, ahash::RandomState>
}

impl Level {
    pub fn new() -> Level {
        Level {
            chunks: DashMap::with_hasher(ahash::RandomState::new())
        }
    }
    pub fn add_chunk(&mut self, key: ChunkCoord, chunk: ChunkType) {
        //println!("added {:?}", key);
        self.chunks.insert(key,chunk);
    }
    // pub fn get_full(&self, key:&ChunkCoord) -> Option<&Chunk> {
    //     if let Some(ctype) = self.chunks.get(key) {
    //         if let ChunkType::Full(chunk) = ctype.value() {
    //             return Some(chunk);
    //         }
    //     }
    //     None
    // }
    // pub fn get_full_mut(&self, key:&ChunkCoord) -> Option<&mut Chunk> {
    //     if let Some(mut ctype) = self.chunks.get_mut(key) {
    //         let v = ctype.value_mut();
    //         return match v {
    //             ChunkType::Ungenerated => None,
    //             ChunkType::Full(x) => Some(x),
    //         }
    //     }
    //     None
    // }
}