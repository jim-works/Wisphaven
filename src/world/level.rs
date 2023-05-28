use bevy::prelude::*;
use dashmap::DashMap;
use super::chunk::*;

#[derive(Resource)]
pub struct Level {
    chunks: DashMap<ChunkCoord, ChunkType, ahash::RandomState>,
    lod_chunks: Vec<DashMap<ChunkCoord, LODChunkType, ahash::RandomState>>,
}

impl Level {
    pub fn new(lod_levels: usize) -> Level {
        Level {
            chunks: DashMap::with_hasher(ahash::RandomState::new()),
            lod_chunks: vec![DashMap::with_hasher(ahash::RandomState::new()); lod_levels]
        }
    }
    pub fn contains_chunk(&self, key:ChunkCoord) -> bool {
        self.chunks.contains_key(&key)
    }
    pub fn chunks_iter(&self) -> dashmap::iter::Iter<'_, ChunkCoord, ChunkType, ahash::RandomState>{
        self.chunks.iter()
    }
    pub fn remove_chunk(&mut self, key: ChunkCoord) -> Option<(ChunkCoord,ChunkType)> {
        self.chunks.remove(&key)
    }
    pub fn get_chunk(&self, key: ChunkCoord) -> Option<dashmap::mapref::one::Ref<'_, ChunkCoord, ChunkType, ahash::RandomState>> {
        self.chunks.get(&key)
    }
    pub fn add_chunk(&mut self, key: ChunkCoord, chunk: ChunkType) {
        self.chunks.insert(key,chunk); 
    }
    pub fn add_lod_chunk(&mut self, key: ChunkCoord, chunk: LODChunkType) {
        match chunk {
            LODChunkType::Ungenerated(_, level) => self.insert_chunk_at_lod(key, level, chunk),
            LODChunkType::Full(l) => self.insert_chunk_at_lod(key, l.level, LODChunkType::Full(l)),
            _ => {}
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
    pub fn get_lod_levels(&self) -> usize {
        self.lod_chunks.len()
    }
    pub fn remove_lod_chunk(&mut self, level: usize, position: ChunkCoord) -> Option<(ChunkCoord,LODChunkType)> {
        //println!("removed lod chunk {}", level);
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