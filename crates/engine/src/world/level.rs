use std::{ops::Deref, sync::Arc};

use crate::{
    mesher::NeedsMesh,
    serialization::{ChunkSaveFormat, NeedsLoading, NeedsSaving},
    util::{
        direction::Direction,
        iterators::{Volume, VolumeContainer},
        max_component_norm,
    },
    world::BlockcastHit,
    worldgen::{ChunkNeedsGenerated, GeneratedChunk, GenerationPhase},
};
use bevy::{prelude::*, utils::hashbrown::HashSet};
use dashmap::DashMap;

use super::{
    chunk::*,
    events::{BlockDamageSetEvent, BlockUsedEvent, ChunkUpdatedEvent},
    BlockBuffer, BlockCoord, BlockDamage, BlockId, BlockRegistry, BlockType, Id, UsableBlock,
};

#[derive(Resource)]
pub struct Level(pub Arc<LevelData>);

impl AsRef<LevelData> for Level {
    fn as_ref(&self) -> &LevelData {
        &self.0
    }
}

impl Deref for Level {
    type Target = LevelData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//entity is the cracked visual
struct DamagedBlock(BlockDamage, Entity);

pub struct LevelData {
    pub name: &'static str,
    pub seed: u64,
    chunks: DashMap<ChunkCoord, ChunkType, ahash::RandomState>,
    buffers: DashMap<ChunkCoord, Box<[BlockType; BLOCKS_PER_CHUNK]>, ahash::RandomState>,
    block_damages: DashMap<BlockCoord, BlockDamage, ahash::RandomState>,
    lod_chunks:
        DashMap<usize, DashMap<ChunkCoord, LODChunkType, ahash::RandomState>, ahash::RandomState>,
    spawn_point: Vec3,
}

impl LevelData {
    pub fn new(name: &'static str, seed: u64) -> LevelData {
        LevelData {
            name,
            seed,
            chunks: DashMap::with_hasher(ahash::RandomState::new()),
            buffers: DashMap::with_hasher(ahash::RandomState::new()),
            block_damages: DashMap::with_hasher(ahash::RandomState::new()),
            lod_chunks: DashMap::with_hasher(ahash::RandomState::new()),
            spawn_point: Vec3::new(0.0, 0.0, 10.0),
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
                BlockType::Filled(entity) => Some(entity),
            },
            None => None,
        }
    }
    pub fn get_blocks_in_volume(&self, volume: Volume) -> VolumeContainer<BlockType> {
        let mut container = VolumeContainer::new(volume);
        self.fill_volume_container(&mut container);
        container
    }
    pub fn fill_volume_container(&self, container: &mut VolumeContainer<BlockType>) {
        //todo - optimize to get needed chunks all at once
        for pos in container.volume().iter() {
            container.set(pos, self.get_block(pos.into()));
        }
    }
    //adds damage to the block at `key`. damage ranges from 0-1, with 1 destroying the block
    //returns the block's entity if it's destroyed
    //will not do anything if damage = 0
    pub fn damage_block(
        &self,
        key: BlockCoord,
        amount: f32,
        damager: Option<Entity>, //the item, block, or other entity that damaged the block. not the player
        id_query: &Query<&BlockId>,
        writer: &mut EventWriter<BlockDamageSetEvent>,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
        commands: &mut Commands,
    ) -> Option<Entity> {
        let mut remove_block = false;
        let mut remove_damage = false;
        let entity = self.get_block_entity(key);
        if entity.is_none() || amount == 0.0 {
            return None; //can't damage an empty block, or we did literally no damage
        }
        match self.block_damages.get_mut(&key) {
            Some(mut dam) => {
                let mut damage = dam.value().with_time_reset();
                damage.damage = (damage.damage + amount).clamp(0.0, 1.0);
                *dam.value_mut() = damage;
                if damage.damage == 1.0 {
                    //total damage = 1, remove the block
                    remove_block = true;
                } else if damage.damage == 0.0 {
                    //no more damage, so remove the damage value
                    remove_damage = true;
                }
                writer.send(BlockDamageSetEvent {
                    block_position: key,
                    damage,
                    damager,
                });
            }
            None => {
                if amount < 1.0 {
                    let damage = BlockDamage::new(amount);
                    self.block_damages.insert(key, damage);
                    writer.send(BlockDamageSetEvent {
                        block_position: key,
                        damage,
                        damager,
                    });
                } else {
                    remove_block = true;
                    writer.send(BlockDamageSetEvent {
                        block_position: key,
                        damage: BlockDamage::new(1.0),
                        damager,
                    });
                }
            }
        }
        if remove_block || remove_damage {
            self.block_damages.remove(&key);
        }
        if remove_block {
            self.set_block_entity(key, BlockType::Empty, id_query, update_writer, commands);
            return entity;
        }
        None
    }
    //heals all block damages by amount
    pub fn heal_block_damages(
        &self,
        seconds_elapsed: f32,
        writer: &mut EventWriter<BlockDamageSetEvent>,
    ) {
        self.block_damages.retain(|key, damage| {
            damage.seconds_to_next_heal -= seconds_elapsed;
            if damage.seconds_to_next_heal <= 0.0 {
                damage.damage = (damage.damage - BlockDamage::HEAL_PER_TICK).clamp(0.0, 1.0);
                *damage = damage.with_time_reset();
                writer.send(BlockDamageSetEvent {
                    block_position: *key,
                    damage: *damage,
                    damager: None,
                });
            }
            damage.damage > 0.0
        });
    }
    //returns true if the targeted block could be used, false otherwise
    pub fn use_block(
        &self,
        key: BlockCoord,
        user: Entity,
        use_forward: Dir3,
        query: &Query<&UsableBlock>,
        writer: &mut EventWriter<BlockUsedEvent>,
    ) -> bool {
        match self.get_block_entity(key) {
            Some(entity) => match query.get(entity) {
                Ok(_) => {
                    writer.send(BlockUsedEvent {
                        block_position: key,
                        user,
                        use_forward,
                        block_used: entity,
                    });
                    true
                }
                Err(_) => false,
            },
            None => false,
        }
    }
    //doesn't mesh or update physics
    pub fn set_block_noupdate(
        &self,
        key: BlockCoord,
        val: BlockId,
        registry: &BlockRegistry,
        id_query: &Query<&BlockId>,
        commands: &mut Commands,
    ) -> Option<Entity> {
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
    pub fn set_block_entity_noupdate(
        &self,
        key: BlockCoord,
        val: BlockType,
        id_query: &Query<&BlockId>,
        commands: &mut Commands,
    ) -> Option<Entity> {
        if let Some(mut r) = self.get_chunk_mut(ChunkCoord::from(key)) {
            if let ChunkType::Full(ref mut chunk) = r.value_mut() {
                BlockRegistry::remove_entity(id_query, chunk[ChunkIdx::from(key)], commands);
                ChunkTrait::set_block(chunk, ChunkIdx::from(key).into(), val);
                return Some(chunk.entity);
            }
        }
        None
    }
    pub fn update_chunk_only<const SAVE: bool>(
        chunk_entity: Entity,
        coord: ChunkCoord,
        commands: &mut Commands,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
    ) {
        if SAVE {
            if let Some(mut ec) = commands.get_entity(chunk_entity) {
                ec.try_insert((NeedsMesh::default(), NeedsSaving));
            }
        } else {
            if let Some(mut ec) = commands.get_entity(chunk_entity) {
                ec.try_insert(NeedsMesh::default());
            }
        }
        update_writer.send(ChunkUpdatedEvent { coord });
    }
    pub fn update_chunk_neighbors_only(
        &self,
        coord: ChunkCoord,
        commands: &mut Commands,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
    ) {
        for dir in Direction::iter() {
            if let Some(neighbor_ref) = self.get_chunk(coord.offset(dir)) {
                if let ChunkType::Full(c) = neighbor_ref.value() {
                    if let Some(mut ec) = commands.get_entity(c.entity) {
                        ec.try_insert(NeedsMesh::default());
                    }
                    update_writer.send(ChunkUpdatedEvent { coord: c.position });
                }
            }
        }
    }
    pub fn set_block(
        &self,
        key: BlockCoord,
        val: BlockId,
        registry: &BlockRegistry,
        id_query: &Query<&BlockId>,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
        commands: &mut Commands,
    ) {
        match val {
            id @ BlockId(Id::Basic(_)) | id @ BlockId(Id::Dynamic(_)) => {
                if let Some(entity) = registry.generate_entity(val, key, commands) {
                    self.set_block_entity(
                        key,
                        BlockType::Filled(entity),
                        id_query,
                        update_writer,
                        commands,
                    );
                } else {
                    error!("Tried to set a block with id: {:?} that has no entity!", id);
                }
            }
            BlockId(Id::Empty) => {
                self.set_block_entity(key, BlockType::Empty, id_query, update_writer, commands)
            }
        }
    }
    pub fn batch_set_block<I: Iterator<Item = (BlockCoord, BlockId)>>(
        &self,
        to_set: I,
        registry: &BlockRegistry,
        id_query: &Query<&BlockId>,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
        commands: &mut Commands,
    ) {
        let _my_span = info_span!(
            "batch_set_block_entities",
            name = "batch_set_block_entities"
        )
        .entered();
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
                Self::update_chunk_only::<true>(entity, chunk_coord, commands, update_writer);
            }
        }
    }
    //updates chunk and neighbors
    pub fn set_block_entity(
        &self,
        key: BlockCoord,
        val: BlockType,
        id_query: &Query<&BlockId>,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
        commands: &mut Commands,
    ) {
        self.batch_set_block_entities(
            std::iter::once((key, val)),
            id_query,
            update_writer,
            commands,
        );
    }
    //meshes and updates physics
    pub fn batch_set_block_entities<I: Iterator<Item = (BlockCoord, BlockType)>>(
        &self,
        to_set: I,
        id_query: &Query<&BlockId>,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
        commands: &mut Commands,
    ) {
        let _my_span = info_span!(
            "batch_set_block_entities",
            name = "batch_set_block_entities"
        )
        .entered();
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
                Self::update_chunk_only::<true>(entity, chunk_coord, commands, update_writer);
            }
        }
    }
    //overwrites the chunk data, minus the entity, with the provided chunk
    //does not automatically update
    //spawns a new chunk if one doesn't already exist at `coord`
    pub fn overwrite_or_spawn_chunk(
        &self,
        coord: ChunkCoord,
        chunk: ChunkSaveFormat,
        commands: &mut Commands,
        registry: &BlockRegistry,
    ) -> Entity {
        //overwrite old chunk
        if let Some(mut r) = self.get_chunk_mut(coord) {
            let v = r.value_mut();
            let id = match v {
                ChunkType::Ungenerated(e) => {
                    let id = *e;
                    *v = ChunkType::Full(chunk.into_chunk(id, registry, commands));
                    id
                }
                ChunkType::Generating(_, c) => {
                    let id = c.entity;
                    *v = ChunkType::Full(chunk.into_chunk(id, registry, commands));
                    id
                }
                ChunkType::Full(c) => {
                    let id = c.entity;
                    *v = ChunkType::Full(chunk.into_chunk(id, registry, commands));
                    id
                }
            };
            return id;
        }
        //spawn new chunk
        let id = commands
            .spawn((
                GeneratedChunk,
                Transform::from_translation(coord.to_vec3()),
                Visibility::default(),
                coord,
                Name::new("Chunk"),
            ))
            .id();
        self.add_chunk(
            coord,
            ChunkType::Full(chunk.into_chunk(id, registry, commands)),
        );
        id
    }
    pub fn load_chunk(
        &self,
        coord: ChunkCoord,
        should_mesh: bool,
        commands: &mut Commands,
    ) -> Entity {
        let id = match self.get_chunk_entity(coord) {
            Some(id) => id,
            None => {
                let id = commands
                    .spawn((
                        Name::new("Chunk"),
                        coord,
                        Transform::default(),
                        Visibility::default(),
                        NeedsLoading,
                    ))
                    .id();
                self.add_chunk(coord, ChunkType::Ungenerated(id));
                id
            }
        };
        if should_mesh {
            if let Some(mut ec) = commands.get_entity(id) {
                ec.remove::<DontMeshChunk>();
            }
        } else {
            if let Some(mut ec) = commands.get_entity(id) {
                ec.try_insert(DontMeshChunk);
            }
        }
        id
    }
    pub fn create_lod_chunk(&self, coord: ChunkCoord, lod_level: u8, commands: &mut Commands) {
        let id = commands
            .spawn((
                Name::new("LODChunk"),
                coord,
                Transform::default(),
                Visibility::default(),
                ChunkNeedsGenerated::Lod(lod_level),
            ))
            .id();
        self.add_lod_chunk(
            coord,
            crate::world::chunk::LODChunkType::Ungenerated(id, lod_level),
        );
    }
    pub fn add_buffer(
        &self,
        buffer: BlockBuffer<BlockType>,
        commands: &mut Commands,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
    ) {
        let _my_span = info_span!("add_buffer", name = "add_buffer").entered();
        for (coord, buf) in buffer.buf {
            //if the chunk is already generated, add the contents of the buffer to the chunk
            if let Some(mut chunk_ref) = self.get_chunk_mut(coord) {
                if let ChunkType::Full(ref mut c) = chunk_ref.value_mut() {
                    buf.apply_to(c.blocks.as_mut());
                    Self::update_chunk_only::<true>(c.entity, c.position, commands, update_writer);
                    //self.update_chunk_neighbors_only(c.position, commands);
                    continue;
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
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
    ) {
        let _my_span = info_span!("add_array_buffer", name = "add_array_buffer").entered();
        //if the chunk is already generated, add the contents of the buffer to the chunk
        if let Some(mut chunk_ref) = self.get_chunk_mut(coord) {
            if let ChunkType::Full(ref mut c) = chunk_ref.value_mut() {
                let mut start = 0;
                for (block, run) in buf {
                    if !matches!(*block, BlockType::Empty) {
                        for i in start..start + *run as usize {
                            c.set_block(i, *block);
                        }
                    }
                    start += *run as usize;
                }
                Self::update_chunk_only::<true>(c.entity, c.position, commands, update_writer);
                //self.update_chunk_neighbors_only(c.position, commands);
                //we've already spawned in the buffer, so we shouldn't store it
                return;
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
                for stored_block in stored_buf.iter_mut().skip(start).take(*run as usize) {
                    *stored_block = *block;
                }
            }
            start += *run as usize;
        }
    }
    pub fn get_buffer(
        &self,
        key: &ChunkCoord,
    ) -> Option<dashmap::mapref::one::Ref<'_, ChunkCoord, Box<[BlockType; BLOCKS_PER_CHUNK]>>> {
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
    ) -> Option<dashmap::mapref::one::Ref<'_, ChunkCoord, ChunkType>> {
        self.chunks.get(&key)
    }
    pub fn get_chunk_mut(
        &self,
        key: ChunkCoord,
    ) -> Option<dashmap::mapref::one::RefMut<'_, ChunkCoord, ChunkType>> {
        self.chunks.get_mut(&key)
    }
    pub fn get_chunk_entity(&self, key: ChunkCoord) -> Option<Entity> {
        if let Some(r) = self.get_chunk(key) {
            return match r.value() {
                ChunkType::Ungenerated(id) => Some(*id),
                ChunkType::Generating(_, chunk) => Some(chunk.entity),
                ChunkType::Full(chunk) => Some(chunk.entity),
            };
        }
        None
    }
    pub fn update_chunk_phase(&self, key: ChunkCoord, phase: GenerationPhase) {
        let _my_span = info_span!("update_chunk_phase", name = "update_chunk_phase").entered();
        if let Some(mut c) = self.get_chunk_mut(key) {
            if let ChunkType::Generating(old_phase, _) = c.value_mut() {
                if phase > *old_phase {
                    *old_phase = phase;
                }
            }
        }
    }
    pub fn promote_generating_to_full(
        &self,
        key: ChunkCoord,
        registry: &BlockRegistry,
        commands: &mut Commands,
    ) {
        let _my_span = info_span!(
            "promote_generating_to_full",
            name = "promote_generating_to_full"
        )
        .entered();
        if let Some(mut c) = self.get_chunk_mut(key) {
            if let ChunkType::Generating(_, chunk) = c.value_mut() {
                *c = ChunkType::Full(chunk.to_array_chunk(registry, commands));
                self.apply_buffer(c.value_mut())
            }
        }
    }
    /// Replaces the `Chunk` at `key` with `chunk`.
    /// If `chunk` is `ChunkType::Full`: removes the chunk's buffer and merges it with the chunk
    pub fn add_chunk(&self, key: ChunkCoord, mut chunk: ChunkType) {
        let _my_span = info_span!("add_chunk", name = "add_chunk").entered();
        //copy contents of buffer into chunk if necessary
        self.apply_buffer(&mut chunk);
        self.chunks.insert(key, chunk);
    }
    pub fn apply_buffer(&self, chunk: &mut ChunkType) {
        if let ChunkType::Full(ref mut c) = chunk {
            if let Some((_, buf)) = self.buffers.remove(&c.position) {
                for i in 0..BLOCKS_PER_CHUNK {
                    if !matches!(buf[i], BlockType::Empty) {
                        c.set_block(i, buf[i]);
                    }
                }
            }
        }
    }
    pub fn add_lod_chunk(&self, key: ChunkCoord, chunk: LODChunkType) {
        let _my_span = info_span!("add_lod_chunk", name = "add_lod_chunk").entered();
        match chunk {
            LODChunkType::Ungenerated(_, level) => {
                self.insert_chunk_at_lod(key, level as usize, chunk)
            }
            LODChunkType::Full(l) => {
                self.insert_chunk_at_lod(key, l.level as usize, LODChunkType::Full(l))
            }
        }
    }
    fn insert_chunk_at_lod(&self, key: ChunkCoord, level: usize, chunk: LODChunkType) {
        //expand lod vec if needed
        if self.lod_chunks.len() <= level {
            for x in self.lod_chunks.len()..level + 1 {
                self.lod_chunks
                    .insert(x, DashMap::with_hasher(ahash::RandomState::new()));
            }
        }
        self.lod_chunks.get(&level).unwrap().insert(key, chunk);
    }
    pub fn get_lod_chunks(
        &self,
        level: usize,
    ) -> Option<
        dashmap::mapref::one::Ref<'_, usize, DashMap<ChunkCoord, LODChunkType, ahash::RandomState>>,
    > {
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

    //todo improve this (bresehams -> sweep_ray)
    //only hits blocks
    pub fn blockcast(
        &self,
        origin: Vec3,
        end_offset: Vec3,
        mut checker: impl FnMut(Option<BlockType>) -> bool,
    ) -> Option<BlockcastHit> {
        let _my_span = info_span!("blockcast", name = "blockcast").entered();
        const STEP_SIZE: f32 = 1.0 / 32.0;
        let line_len = end_offset.length();
        let line_norm = end_offset / line_len;
        let mut old_coords = BlockCoord::from(origin);
        let block = self.get_block(old_coords);
        if checker(block) {
            return Some(BlockcastHit {
                hit_pos: origin,
                block_pos: old_coords,
                block,
                normal: BlockCoord::new(0, 0, 0),
            });
        }
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
            if checker(b) {
                return Some(BlockcastHit {
                    hit_pos: test_point,
                    block_pos: test_block,
                    block: b,
                    normal: max_component_norm(test_point - old_coords.center()).into(),
                });
            }
        }
        None
    }

    pub fn get_spawn_point(&self) -> Vec3 {
        let mut calculated_spawn_point: IVec3 = self.spawn_point.as_ivec3();
        //checks for a 3x3x3 area of air above a 3x1x3 volume that contains at least one non-air block
        // we will check `CHECK_UP_RANGE` spawns iterating in the +Y direction, then return to where we started and `CHECK_DOWN_RANGE` spawns in the -Y direction
        // this repeats from each end `MAX_CHECKS` times.
        //the idea is to prefer spawning above ground, but prefer spawning in the shallow cave than some place way up in the sky.
        const MIN_SPAWN_VOLUME: IVec3 = IVec3::splat(3);
        const MAX_CHECKS: i32 = 100;
        const CHECK_UP_RANGE: i32 = 100;
        const CHECK_DOWN_RANGE: i32 = 50;
        for dy in (0..MAX_CHECKS)
            .map(|i| (0..i * CHECK_UP_RANGE).chain((-i * CHECK_DOWN_RANGE..0).rev()))
            .flatten()
        {
            calculated_spawn_point.y = dy;
            let ground_volume = Volume::new_inclusive(
                calculated_spawn_point + IVec3::NEG_Y,
                calculated_spawn_point + IVec3::new(MIN_SPAWN_VOLUME.x, -1, MIN_SPAWN_VOLUME.z),
            );
            let found_ground = ground_volume
                .iter()
                .any(|coord| matches!(self.get_block(coord.into()), Some(BlockType::Filled(_))));
            if !found_ground {
                continue;
            }
            let air_volume = Volume::new_inclusive(
                calculated_spawn_point + IVec3::Y,
                calculated_spawn_point + MIN_SPAWN_VOLUME,
            );
            let enough_air = air_volume
                .iter()
                .all(|coord| matches!(self.get_block(coord.into()), Some(BlockType::Empty) | None));
            if !enough_air {
                continue;
            }
            info!("found spawn at {:?}", calculated_spawn_point);
            break;
        }
        let spawn_point = calculated_spawn_point.as_vec3() + MIN_SPAWN_VOLUME.as_vec3() / 2.;
        info!("spawn point is {:?}", spawn_point);
        spawn_point
    }
    pub fn get_initial_spawn_point(&self) -> Vec3 {
        self.spawn_point
    }
}
