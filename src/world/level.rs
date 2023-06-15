use crate::{
    mesher::NeedsMesh,
    physics::NeedsPhysics,
    util::{max_component_norm, Direction},
    world::BlockcastHit,
};
use bevy::{prelude::*, utils::hashbrown::HashSet};
use dashmap::DashMap;

use super::{chunk::*, BlockBuffer, BlockCoord, BlockType};

#[derive(Resource)]
pub struct Level {
    pub name: String,
    pub spawn_point: Vec3,
    chunks: DashMap<ChunkCoord, ChunkType, ahash::RandomState>,
    buffers: DashMap<ChunkCoord, Box<[BlockType; BLOCKS_PER_CHUNK]>, ahash::RandomState>,
    lod_chunks: Vec<DashMap<ChunkCoord, LODChunkType, ahash::RandomState>>,
}

impl Level {
    pub fn new(name: String, lod_levels: usize) -> Level {
        Level {
            name,
            chunks: DashMap::with_hasher(ahash::RandomState::new()),
            buffers: DashMap::with_hasher(ahash::RandomState::new()),
            lod_chunks: vec![DashMap::with_hasher(ahash::RandomState::new()); lod_levels],
            spawn_point: Vec3::ZERO
        }
    }
    pub fn get_block(&self, key: BlockCoord) -> Option<BlockType> {
        if let Some(r) = self.get_chunk(ChunkCoord::from(key)) {
            if let ChunkType::Full(chunk) = r.value() {
                return Some(chunk[ChunkIdx::from(key)]);
            }
        }
        None
    }
    //doesn't mesh or update physics
    pub fn set_block_noupdate(&self, key: BlockCoord, val: BlockType) -> Option<Entity> {
        if let Some(mut r) = self.get_chunk_mut(ChunkCoord::from(key)) {
            if let ChunkType::Full(ref mut chunk) = r.value_mut() {
                chunk[ChunkIdx::from(key)] = val;
                return Some(chunk.entity);
            }
        }
        None
    }
    pub fn update_chunk_only(chunk_entity: Entity, commands: &mut Commands) {
        commands
            .entity(chunk_entity)
            .insert((NeedsMesh {}, NeedsPhysics {}));
    }
    pub fn update_chunk_neighbors_only(&self, coord: ChunkCoord, commands: &mut Commands) {
        for dir in Direction::iter() {
            if let Some(neighbor_ref) = self.get_chunk(coord.offset(dir)) {
                match neighbor_ref.value() {
                    ChunkType::Full(c) => {
                        commands
                            .entity(c.entity)
                            .insert((NeedsMesh {}, NeedsPhysics {}));
                    }
                    ChunkType::Ungenerated(entity) => {
                        commands
                            .entity(*entity)
                            .insert((NeedsMesh {}, NeedsPhysics {}));
                    }
                }
            }
        }
    }
    //updates chunk and neighbors
    pub fn set_block(&self, key: BlockCoord, val: BlockType, commands: &mut Commands) {
        self.batch_set_block(std::iter::once((key, val)), commands);
    }
    //meshes and updates physics
    pub fn batch_set_block<I: Iterator<Item = (BlockCoord, BlockType)>>(
        &self,
        to_set: I,
        commands: &mut Commands,
    ) {
        let _my_span = info_span!("batch_set_block", name = "batch_set_block").entered();
        let mut to_update = HashSet::new();
        for (coord, block) in to_set {
            let chunk_coord: ChunkCoord = coord.into();
            //add chunk and neighbors
            to_update.insert(chunk_coord);
            for dir in Direction::iter() {
                to_update.insert(chunk_coord.offset(dir));
            }
            self.set_block_noupdate(coord, block);
        }
        //update chunk info: meshes and physics
        for chunk_coord in to_update {
            if let Some(entity) = self.get_chunk_entity(chunk_coord) {
                Self::update_chunk_only(entity, commands);
            }
        }
    }
    pub fn add_buffer(&self, buffer: BlockBuffer, commands: &mut Commands) {
        let _my_span = info_span!("add_buffer", name = "add_buffer").entered();
        for (coord, buf) in buffer.buf {
            //if the chunk is already generated, add the contents of the buffer to the chunk
            if let Some(mut chunk_ref) = self.get_chunk_mut(coord) {
                match chunk_ref.value_mut() {
                    ChunkType::Ungenerated(_) => {}
                    ChunkType::Full(ref mut c) => {
                        buf.apply_to(c);
                        Self::update_chunk_only(c.entity, commands);
                        //self.update_chunk_neighbors_only(c.position, commands);
                        continue;
                    }
                }
            }
            //we break if we updated a chunk in the world, so now we merge the buffer
            //TODO: figure out how to remove this allocation (must keep mutable reference alive for locking)
            let mut entry = self
                .buffers
                .entry(coord)
                .or_insert(Box::new([BlockType::Empty; 4096]));
            //copy contents of buf into entry, since they are different buffers
            buf.apply_to(entry.value_mut().as_mut());
        }
    }
    pub fn contains_chunk(&self, key: ChunkCoord) -> bool {
        self.chunks.contains_key(&key)
    }
    pub fn chunks_iter(
        &self,
    ) -> dashmap::iter::Iter<'_, ChunkCoord, ChunkType, ahash::RandomState> {
        self.chunks.iter()
    }
    pub fn remove_chunk(&self, key: ChunkCoord) -> Option<(ChunkCoord, ChunkType)> {
        self.chunks.remove(&key)
    }
    pub fn get_chunk(
        &self,
        key: ChunkCoord,
    ) -> Option<dashmap::mapref::one::Ref<'_, ChunkCoord, ChunkType, ahash::RandomState>> {
        self.chunks.get(&key)
    }
    pub fn get_chunk_mut(
        &self,
        key: ChunkCoord,
    ) -> Option<dashmap::mapref::one::RefMut<'_, ChunkCoord, ChunkType, ahash::RandomState>> {
        self.chunks.get_mut(&key)
    }
    pub fn get_chunk_entity(&self, key: ChunkCoord) -> Option<Entity> {
        if let Some(r) = self.get_chunk(ChunkCoord::from(key)) {
            if let ChunkType::Full(chunk) = r.value() {
                return Some(chunk.entity);
            } else if let ChunkType::Ungenerated(e) = r.value() {
                return Some(*e);
            }
        }
        None
    }
    pub fn add_chunk(&self, key: ChunkCoord, chunk: ChunkType) {
        let _my_span = info_span!("add_chunk", name = "add_chunk").entered();
        //copy contents of buffer into chunk if necessary
        if let ChunkType::Full(mut c) = chunk {
            if let Some((_, buf)) = self.buffers.remove(&key) {
                for i in 0..BLOCKS_PER_CHUNK {
                    if !matches!(buf[i], BlockType::Empty) {
                        c[i] = buf[i];
                    }
                }
            }
            self.chunks.insert(key, ChunkType::Full(c));
        } else {
            self.chunks.insert(key, chunk);
        }
    }
    pub fn add_lod_chunk(&mut self, key: ChunkCoord, chunk: LODChunkType) {
        let _my_span = info_span!("add_lod_chunk", name = "add_lod_chunk").entered();
        match chunk {
            LODChunkType::Ungenerated(_, level) => self.insert_chunk_at_lod(key, level, chunk),
            LODChunkType::Full(l) => self.insert_chunk_at_lod(key, l.level, LODChunkType::Full(l)),
        }
    }
    fn insert_chunk_at_lod(&mut self, key: ChunkCoord, level: usize, chunk: LODChunkType) {
        //expand lod vec if needed
        if self.lod_chunks.len() <= level {
            let iterations = level - self.lod_chunks.len() + 1;
            for _ in 0..iterations {
                self.lod_chunks
                    .push(DashMap::with_hasher(ahash::RandomState::new()))
            }
        }
        self.lod_chunks[level].insert(key, chunk);
    }
    pub fn get_lod_chunks(
        &self,
        level: usize,
    ) -> Option<&DashMap<ChunkCoord, LODChunkType, ahash::RandomState>> {
        self.lod_chunks.get(level)
    }
    pub fn get_lod_levels(&self) -> usize {
        self.lod_chunks.len()
    }
    pub fn remove_lod_chunk(
        &mut self,
        level: usize,
        position: ChunkCoord,
    ) -> Option<(ChunkCoord, LODChunkType)> {
        //println!("removed lod chunk {}", level);
        match self.lod_chunks.get(level) {
            None => None,
            Some(map) => map.remove(&position),
        }
    }
    pub fn contains_lod_chunk(&self, level: usize, position: ChunkCoord) -> bool {
        match self.lod_chunks.get(level) {
            None => false,
            Some(map) => map.contains_key(&position),
        }
    }
    //todo improve this (bresenham's?)
    pub fn blockcast(&self, origin: Vec3, line: Vec3) -> Option<BlockcastHit> {
        let _my_span = info_span!("blockcast", name = "blockcast").entered();
        const STEP_SIZE: f32 = 0.05;
        let line_len = line.length();
        let line_norm = line / line_len;
        let mut old_coords = BlockCoord::from(origin);
        match self.get_block(old_coords) {
            Some(BlockType::Empty) | None => {}
            Some(t) => {
                return Some(BlockcastHit {
                    hit_pos: origin,
                    block_pos: old_coords,
                    block: t,
                    normal: BlockCoord::new(0, 0, 0),
                })
            }
        };
        let mut t = 0.0;
        while t < line_len {
            t += STEP_SIZE;
            let test_point = origin + t * line_norm;
            let test_block = BlockCoord::from(test_point);
            if test_block == old_coords {
                continue;
            }

            old_coords = test_block;
            let b = self.get_block(test_block);
            match b {
                Some(BlockType::Empty) | None => {}
                Some(t) => {
                    return Some(BlockcastHit {
                        hit_pos: test_point,
                        block_pos: test_block,
                        block: t,
                        normal: max_component_norm(test_point - old_coords.center()).into(),
                    })
                }
            }
        }
        None
    }
}
