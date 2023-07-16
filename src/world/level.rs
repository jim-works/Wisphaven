use std::{sync::Arc, ops::Deref};

use crate::{
    mesher::NeedsMesh,
    physics::NeedsPhysics,
    serialization::{NeedsLoading, NeedsSaving},
    util::{max_component_norm, Direction},
    world::BlockcastHit,
    worldgen::ChunkNeedsGenerated,
};
use bevy::{prelude::*, utils::hashbrown::HashSet};
use dashmap::DashMap;

use super::{chunk::*, BlockBuffer, BlockCoord, BlockType, BlockId, BlockRegistry, events::BlockUsedEvent, UsableBlock, Id};

#[derive(Resource)]
pub struct Level(pub Arc<LevelData>);

impl AsRef<LevelData> for Level {
    fn as_ref(&self) -> &LevelData {
        self.0.as_ref()
    }
}

impl Deref for Level {
    type Target = LevelData;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

pub struct LevelData {
    pub name: &'static str,
    pub spawn_point: Vec3,
    pub seed: u64,
    chunks: DashMap<ChunkCoord, ChunkType, ahash::RandomState>,
    buffers: DashMap<ChunkCoord, Box<[BlockType; BLOCKS_PER_CHUNK]>, ahash::RandomState>,
    lod_chunks: DashMap<usize, DashMap<ChunkCoord, LODChunkType, ahash::RandomState>, ahash::RandomState>,
}

impl LevelData {
    pub fn new(name: &'static str, seed: u64) -> LevelData {
        LevelData {
            name,
            seed,
            chunks: DashMap::with_hasher(ahash::RandomState::new()),
            buffers: DashMap::with_hasher(ahash::RandomState::new()),
            lod_chunks: DashMap::with_hasher(ahash::RandomState::new()),
            spawn_point: Vec3::new(0.0,10.0,0.0),
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
    pub fn get_block_entity(&self, key: BlockCoord) -> Option<Entity> {
        match self.get_block(key) {
            Some(block_type) => match block_type {
                BlockType::Empty => None,
                BlockType::Filled(entity) => Some(entity)
            },
            None => None,
        }
    }
    //returns true if the targeted block could be used, false otherwise
    pub fn use_block(&self, key: BlockCoord, user: Entity, query: &Query<&UsableBlock>, writer: &mut EventWriter<BlockUsedEvent>) -> bool {
        match self.get_block_entity(key) {
            Some(entity) => match query.get(entity) {
                Ok(_) => {
                    writer.send(BlockUsedEvent {
                        block_position: key,
                        user,
                        block_used: entity
                    });
                    true
                },
                Err(_) => false,
            },
            None => false,
        }
    }
    //doesn't mesh or update physics
    pub fn set_block_noupdate(&self, key: BlockCoord, val: BlockId, registry: &BlockRegistry, id_query: &Query<&BlockId>, commands: &mut Commands) -> Option<Entity> {
        if let Some(mut r) = self.get_chunk_mut(ChunkCoord::from(key)) {
            if let ChunkType::Full(ref mut chunk) = r.value_mut() {
                let block = match registry.generate_entity(val, key, commands) {
                    Some(entity) => BlockType::Filled(entity),
                    None => BlockType::Empty,
                };
                BlockRegistry::remove_entity(id_query, chunk[ChunkIdx::from(key)], commands);
                ChunkTrait::set_block(chunk, ChunkIdx::from(key).into(), block);
                return Some(chunk.entity);
            }
        }
        None
    }
    //doesn't mesh or update physics
    pub fn set_block_entity_noupdate(&self, key: BlockCoord, val: BlockType, id_query: &Query<&BlockId>, commands: &mut Commands) -> Option<Entity> {
        if let Some(mut r) = self.get_chunk_mut(ChunkCoord::from(key)) {
            if let ChunkType::Full(ref mut chunk) = r.value_mut() {
                BlockRegistry::remove_entity(id_query, chunk[ChunkIdx::from(key)], commands);
                ChunkTrait::set_block(chunk, ChunkIdx::from(key).into(), val);
                return Some(chunk.entity);
            }
        }
        None
    }
    pub fn update_chunk_only<const SAVE: bool>(chunk_entity: Entity, commands: &mut Commands) {
        if SAVE {
            commands
                .entity(chunk_entity)
                .insert((NeedsMesh, NeedsPhysics, NeedsSaving));
        } else {
            commands
                .entity(chunk_entity)
                .insert((NeedsMesh, NeedsPhysics));
        }
    }
    pub fn update_chunk_neighbors_only(&self, coord: ChunkCoord, commands: &mut Commands) {
        for dir in Direction::iter() {
            if let Some(neighbor_ref) = self.get_chunk(coord.offset(dir)) {
                match neighbor_ref.value() {
                    ChunkType::Full(c) => {
                        commands
                            .entity(c.entity)
                            .insert((NeedsMesh {}, NeedsPhysics {}));
                    },
                    _ => {}
                }
            }
        }
    }
    pub fn set_block(&self, key: BlockCoord, val: BlockId, registry: &BlockRegistry, id_query: &Query<&BlockId>, commands: &mut Commands) {
        match val {
            id @ BlockId(Id::Basic(_)) | id @ BlockId(Id::Dynamic(_)) => {
                if let Some(entity) = registry.generate_entity(val, key, commands) {
                    self.set_block_entity(key, BlockType::Filled(entity), id_query, commands);
                } else {
                    error!("Tried to set a block with id: {:?} that has no entity!", id);
                }
            },
            BlockId(Id::Empty) => self.set_block_entity(key, BlockType::Empty, id_query, commands)
        }
    }
    pub fn batch_set_block<I: Iterator<Item = (BlockCoord, BlockId)>>(
        &self,
        to_set: I,
        registry: &BlockRegistry,
        id_query: &Query<&BlockId>,
        commands: &mut Commands,
    ) {
        let _my_span = info_span!("batch_set_block_entities", name = "batch_set_block_entities").entered();
        let mut to_update = HashSet::new();
        for (coord, block) in to_set {
            let chunk_coord: ChunkCoord = coord.into();
            //add chunk and neighbors
            to_update.insert(chunk_coord);
            for dir in Direction::iter() {
                to_update.insert(chunk_coord.offset(dir));
            }
            self.set_block_noupdate(coord, block, registry, id_query, commands);
        }
        //update chunk info: meshes and physics
        for chunk_coord in to_update {
            if let Some(entity) = self.get_chunk_entity(chunk_coord) {
                Self::update_chunk_only::<true>(entity, commands);
            }
        }
    }
    //updates chunk and neighbors
    pub fn set_block_entity(&self, key: BlockCoord, val: BlockType, id_query: &Query<&BlockId>, commands: &mut Commands) {
        self.batch_set_block_entities(std::iter::once((key, val)), id_query, commands);
    }
    //meshes and updates physics
    pub fn batch_set_block_entities<I: Iterator<Item = (BlockCoord, BlockType)>>(
        &self,
        to_set: I,
        id_query: &Query<&BlockId>,
        commands: &mut Commands,
    ) {
        let _my_span = info_span!("batch_set_block_entities", name = "batch_set_block_entities").entered();
        let mut to_update = HashSet::new();
        for (coord, block) in to_set {
            let chunk_coord: ChunkCoord = coord.into();
            //add chunk and neighbors
            to_update.insert(chunk_coord);
            for dir in Direction::iter() {
                to_update.insert(chunk_coord.offset(dir));
            }
            self.set_block_entity_noupdate(coord, block, id_query, commands);
        }
        //update chunk info: meshes and physics
        for chunk_coord in to_update {
            if let Some(entity) = self.get_chunk_entity(chunk_coord) {
                Self::update_chunk_only::<true>(entity, commands);
            }
        }
    }
    pub fn create_chunk(&self, coord: ChunkCoord, commands: &mut Commands) {
        let id = commands
            .spawn((
                Name::new("Chunk"),
                coord,
                SpatialBundle::default(),
                NeedsLoading,
            ))
            .id();
        self.add_chunk(coord, ChunkType::Ungenerated(id));
    }
    pub fn create_lod_chunk(
        &self,
        coord: ChunkCoord,
        lod_level: u8,
        commands: &mut Commands,
    ) {
        let id = commands
            .spawn((
                Name::new("LODChunk"),
                coord,
                SpatialBundle::default(),
                ChunkNeedsGenerated::Lod(lod_level),
            ))
            .id();
        self.add_lod_chunk(
            coord,
            crate::world::chunk::LODChunkType::Ungenerated(id, lod_level),
        );
    }
    pub fn add_buffer(&self, buffer: BlockBuffer<BlockType>, commands: &mut Commands) {
        let _my_span = info_span!("add_buffer", name = "add_buffer").entered();
        for (coord, buf) in buffer.buf {
            //if the chunk is already generated, add the contents of the buffer to the chunk
            if let Some(mut chunk_ref) = self.get_chunk_mut(coord) {
                match chunk_ref.value_mut() {
                    ChunkType::Full(ref mut c) => {
                        buf.apply_to(c.blocks.as_mut());
                        Self::update_chunk_only::<true>(c.entity, commands);
                        //self.update_chunk_neighbors_only(c.position, commands);
                        continue;
                    },
                    _ => {}
                }
            }
            //we break if we updated a chunk in the world, so now we merge the buffer
            //TODO: figure out how to remove this allocation (must keep mutable reference alive for locking)
            let mut entry = self
                .buffers
                .entry(coord)
                .or_insert(Box::new([BlockType::Empty; BLOCKS_PER_CHUNK]));
            //copy contents of buf into entry, since they are different buffers
            buf.apply_to(entry.value_mut().as_mut());
        }
    }
    pub fn add_rle_buffer(
        &self,
        coord: ChunkCoord,
        buf: &[(BlockType, u16)],
        commands: &mut Commands,
    ) {
        let _my_span = info_span!("add_array_buffer", name = "add_array_buffer").entered();
        //if the chunk is already generated, add the contents of the buffer to the chunk
        if let Some(mut chunk_ref) = self.get_chunk_mut(coord) {
            match chunk_ref.value_mut() {
                ChunkType::Full(ref mut c) => {
                    let mut start = 0;
                    for (block, run) in buf {
                        if !matches!(*block, BlockType::Empty) {
                            for i in start..start + *run as usize {
                                c.set_block(i, *block);
                            }
                        }
                        start += *run as usize;
                    }
                    Self::update_chunk_only::<true>(c.entity, commands);
                    //self.update_chunk_neighbors_only(c.position, commands);
                    //we've already spawned in the buffer, so we shouldn't store it
                    return;
                },
                _ => {}
            }
        }
        //we break if we updated a chunk in the world, so now we merge the buffer
        let mut entry = self
            .buffers
            .entry(coord)
            .or_insert(Box::new([BlockType::Empty; BLOCKS_PER_CHUNK]));
        //copy contents of buf into entry, since they are different buffers
        let stored_buf = entry.value_mut().as_mut();
        let mut start = 0;
        for (block, run) in buf {
            if !matches!(*block, BlockType::Empty) {
                for i in start..start + *run as usize {
                    stored_buf[i] = *block;
                }
            }
            start += *run as usize;
        }
    }
    pub fn get_buffer(
        &self,
        key: &ChunkCoord,
    ) -> Option<
        dashmap::mapref::one::Ref<
            '_,
            ChunkCoord,
            Box<[BlockType; BLOCKS_PER_CHUNK]>,
            ahash::RandomState,
        >,
    > {
        self.buffers.get(key)
    }
    pub fn buffer_iter(
        &self,
    ) -> dashmap::iter::Iter<'_, ChunkCoord, Box<[BlockType; BLOCKS_PER_CHUNK]>, ahash::RandomState>
    {
        self.buffers.iter()
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
        if let Some(r) = self.get_chunk(key) {
            if let ChunkType::Full(chunk) = r.value() {
                return Some(chunk.entity);
            } else if let ChunkType::Ungenerated(e) = r.value() {
                return Some(*e);
            }
        }
        None
    }
    /// Replaces the `Chunk` at `key` with `chunk`.
    /// If `chunk` is `ChunkType::Full`: removes the chunk's buffer and merges it with the chunk
    pub fn add_chunk(&self, key: ChunkCoord, chunk: ChunkType) {
        let _my_span = info_span!("add_chunk", name = "add_chunk").entered();
        //copy contents of buffer into chunk if necessary
        if let ChunkType::Full(mut c) = chunk {
            if let Some((_, buf)) = self.buffers.remove(&key) {
                for i in 0..BLOCKS_PER_CHUNK {
                    if !matches!(buf[i], BlockType::Empty) {
                        c.set_block(i, buf[i]);
                    }
                }
            }
            self.chunks.insert(key, ChunkType::Full(c));
        } else {
            self.chunks.insert(key, chunk);
        }
    }
    pub fn add_lod_chunk(&self, key: ChunkCoord, chunk: LODChunkType) {
        let _my_span = info_span!("add_lod_chunk", name = "add_lod_chunk").entered();
        match chunk {
            LODChunkType::Ungenerated(_, level) => self.insert_chunk_at_lod(key, level as usize, chunk),
            LODChunkType::Full(l) => self.insert_chunk_at_lod(key, l.level as usize, LODChunkType::Full(l)),
        }
    }
    fn insert_chunk_at_lod(&self, key: ChunkCoord, level: usize, chunk: LODChunkType) {
        //expand lod vec if needed
        if self.lod_chunks.len() <= level {
            for x in self.lod_chunks.len()..level+1 {
                self.lod_chunks
                    .insert(x, DashMap::with_hasher(ahash::RandomState::new()));
            }
        }
        self.lod_chunks.get(&level).unwrap().insert(key, chunk);
    }
    pub fn get_lod_chunks(
        &self,
        level: usize,
    ) -> Option<dashmap::mapref::one::Ref<'_, usize, DashMap<ChunkCoord, LODChunkType, ahash::RandomState>, ahash::RandomState>> {
        self.lod_chunks.get(&level)
    }
    pub fn get_lod_levels(&self) -> usize {
        self.lod_chunks.len()
    }
    pub fn remove_lod_chunk(
        &self,
        level: usize,
        position: ChunkCoord,
    ) -> Option<(ChunkCoord, LODChunkType)> {
        match self.lod_chunks.get(&level) {
            None => None,
            Some(map) => map.remove(&position),
        }
    }
    pub fn contains_lod_chunk(&self, level: usize, position: ChunkCoord) -> bool {
        match self.lod_chunks.get(&level) {
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
