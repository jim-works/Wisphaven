use bevy::prelude::*;
use dashmap::DashMap;
use super::chunk::*;

#[derive(Resource)]
pub struct Level {
    pub chunks: DashMap<ChunkCoord, ChunkType, ahash::RandomState>,
    lod_chunks: Vec<DashMap<ChunkCoord, LODChunkType, ahash::RandomState>>,
}

impl Level {
    pub fn new() -> Level {
        Level {
            chunks: DashMap::with_hasher(ahash::RandomState::new()),
            lod_chunks: Vec::new()
        }
    }
    pub fn add_chunk(&mut self, key: ChunkCoord, chunk: ChunkType) {
        self.chunks.insert(key,chunk); 
    }
    pub fn add_lod_chunk(&mut self, key: ChunkCoord, chunk: LODChunkType) {
        match chunk {
            LODChunkType::Ungenerated(_, level) => self.insert_chunk_at_lod(key, level, chunk),
            LODChunkType::Full(l) => self.insert_chunk_at_lod(key, l.level, LODChunkType::Full(l)),
        }
    }
    fn insert_chunk_at_lod(&mut self, key: ChunkCoord, level: usize, chunk: LODChunkType) {
        //expand lod vec if needed
        if self.lod_chunks.len() <= level {
            let iterations = level-self.lod_chunks.len()+1;
            for _ in 0..iterations {
                self.lod_chunks.push(DashMap::with_hasher(ahash::RandomState::new()))
            }
        }
        self.lod_chunks[level].insert(key, chunk);
    }
    pub fn get_lod_chunks(&self, level: usize) -> Option<&DashMap<ChunkCoord, LODChunkType, ahash::RandomState>> {
        self.lod_chunks.get(level)
    }
    pub fn get_lod_chunks_mut(&mut self, level: usize) -> Option<&mut DashMap<ChunkCoord, LODChunkType, ahash::RandomState>> {
        self.lod_chunks.get_mut(level)
    }
    pub fn get_lod_levels(&self) -> usize {
        self.lod_chunks.len()
    }
    pub fn remove_lod_chunk(&mut self, level: usize, position: ChunkCoord) -> Option<(ChunkCoord,LODChunkType)> {
        match self.lod_chunks.get(level) {
            None => None,
            Some(map) => map.remove(&position)
        }
    }
    pub fn contains_lod_chunk(&self, level: usize, position: ChunkCoord) -> bool {
        match self.lod_chunks.get(level) {
            None => false,
            Some(map) => map.contains_key(&position)
        }
    }
}