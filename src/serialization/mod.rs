use std::path::PathBuf;

use itertools::Itertools;
use serde::{Serialize, Deserialize};

use bevy::{
    prelude::*, utils::HashMap,
};

use crate::world::{
    chunk::{ArrayChunk, ChunkCoord, BLOCKS_PER_CHUNK, ChunkIdx},
    BlockType, LevelLoadState, LevelSystemSet, BlockId, BlockRegistry, BlockCoord, events::CreateLevelEvent, Id,
};



pub struct SerializationPlugin;

mod loading;
pub mod queries;
pub mod db;
pub mod state;
mod save;
mod setup;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(state::SerializationStatePlugin)
            .add_system(setup::on_level_created.in_set(OnUpdate(LevelLoadState::NotLoaded)).run_if(in_state(state::GameLoadState::Done)))
            .add_systems(
                (
                    loading::load_chunk_terrain,
                    loading::queue_terrain_loading,
                    db::tick_db,
                    save::do_saving,
                    save::save_all,
                )
                    .in_set(LevelSystemSet::LoadingAndMain)
            )
            .add_system(db::finish_up.in_set(LevelSystemSet::PostUpdate))
            .insert_resource(setup::load_settings())
            //must apply_system_buffers before load_block_registry gets called because load_terrain_texture creates a resources that is needed in load_block_registry
            .add_startup_systems((setup::load_terrain_texture, setup::load_item_textures, apply_system_buffers, setup::start_loading_blocks, setup::start_loading_items).chain().in_base_set(StartupSet::PreStartup))
            .add_systems((setup::load_block_registry, setup::load_item_registry).in_set(OnUpdate(state::GameLoadState::LoadingAssets)))
            .add_system(create_level.in_schedule(OnEnter(state::GameLoadState::Done)))
            .add_event::<SaveChunkEvent>()
            .add_event::<db::DataFromDBEvent>()
            .insert_resource(SaveTimer(Timer::from_seconds(0.1, TimerMode::Repeating)));
    }
}

fn create_level(mut writer: EventWriter<CreateLevelEvent>) {
    writer.send(CreateLevelEvent { name: "level", seed: 8008135 });
    info!("Sent create level event!");
}

#[derive(Resource)]
pub struct BlockTextureMap(pub HashMap<PathBuf, u32>);

#[derive(Resource)]
pub struct ItemTextureMap(pub HashMap<PathBuf, Handle<Image>>);

#[derive(Resource, Default)]
pub struct SavedToLoadedIdMap<T: Into<Id> + Clone + From<Id> + std::hash::Hash + Eq + PartialEq>{
    pub map: HashMap<T, T>,
    pub max_key_id: u32
}

impl<T: Into<Id> + From<Id> + Clone + std::hash::Hash + Eq + PartialEq> SavedToLoadedIdMap<T> {
    pub fn insert(&mut self, key: T, val: T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => {},
            Id::Basic(id) | Id::Dynamic(id) => self.max_key_id = self.max_key_id.max(id),
        }
        self.map.insert(key, val)
    }
    pub fn get(&self, key: &T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => Some(T::from(Id::Empty)),
            _ => self.map.get(key).cloned()
        }
    }
}

#[derive(Resource, Default)]
pub struct LoadedToSavedIdMap<T: Into<Id> + Clone + From<Id> + std::hash::Hash + Eq + PartialEq>{
    pub map: HashMap<T, T>,
    pub max_key_id: u32
}

impl<T: Into<Id> + From<Id> + std::hash::Hash + Clone + Eq + PartialEq> LoadedToSavedIdMap<T> {
    pub fn insert(&mut self, key: T, val: T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => {},
            Id::Basic(id) | Id::Dynamic(id) => self.max_key_id = self.max_key_id.max(id),
        }
        self.map.insert(key, val)
    }
    pub fn get(&self, key: &T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => Some(T::from(Id::Empty)),
            _ => self.map.get(key).cloned()
        }
    }
}

#[derive(Component)]
pub struct NeedsSaving;

#[derive(Component)]
pub struct NeedsLoading;

#[derive(Component)]
pub struct LoadingBlocks;

#[derive(Component)]
pub struct LoadingItems;

#[derive(Resource)]
pub struct SaveTimer(Timer);

pub struct SaveChunkEvent(ChunkCoord);

//run length encoded format for chunks
//TODO: figure out how to do entities
#[derive(Serialize, Deserialize)]
pub struct ChunkSaveFormat {
    pub position: ChunkCoord,
    pub data: Vec<(BlockId, u16)>,
}

#[derive(Debug)]
pub enum ChunkSerializationError {
    InvalidCoordinateFormat,
    InavlidBlockType(u8),
    PanicReading,
}

impl std::fmt::Display for ChunkSerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkSerializationError::InvalidCoordinateFormat => {
                write!(f, "Invalid coordinate format")
            }
            ChunkSerializationError::InavlidBlockType(t) => write!(f, "Invalid block type: {}", t),
            ChunkSerializationError::PanicReading => write!(f, "Panic reading chunk"),
        }
    }
}

impl std::error::Error for ChunkSerializationError {}

impl From<(ChunkCoord, &[BlockId; BLOCKS_PER_CHUNK])> for ChunkSaveFormat {
    fn from(value: (ChunkCoord, &[BlockId; BLOCKS_PER_CHUNK])) -> Self {
        let data = value
            .1
            .iter()
            .dedup_with_count()
            .map(|(run, block)| (*block, run as u16))
            .collect();
        Self {
            position: value.0,
            data,
        }
    }
}

impl ChunkSaveFormat {
    //creates a save format by extracting the ids from the block array using the provided query
    //will replace with the empty block if the entities in the block array do not have a BlockId component
    pub fn ids_only(value: (ChunkCoord, &[BlockType; BLOCKS_PER_CHUNK]), query: &Query<&BlockId>, map: &LoadedToSavedIdMap<BlockId>) -> Self {
                let data = value
            .1
            .iter()
            .dedup_with_count()
            .map(|(run, block)| (match block {
                BlockType::Empty => BlockId(Id::Empty),
                BlockType::Filled(entity) => map.get(query.get(*entity).unwrap_or(&BlockId(Id::Empty))).unwrap(),
            }, run as u16))
            .collect();
        Self {
            position: value.0,
            data,
        }
    }
    pub fn into_chunk(self, chunk_entity: Entity, registry: &BlockRegistry, commands: &mut Commands) -> ArrayChunk {
        let mut curr_idx = 0;
        let mut chunk = ArrayChunk::new(self.position, chunk_entity);
        for (block, length) in self.data.into_iter() {
            for idx in curr_idx..curr_idx + length as usize {
                chunk.blocks[idx] = registry.get_block_type(block, BlockCoord::from(self.position)+BlockCoord::from(ChunkIdx::from_usize(idx)), commands);
            }
            curr_idx += length as usize;
        }
        chunk
    }
    pub fn into_buffer(self, registry: &BlockRegistry, commands: &mut Commands) -> Vec<(BlockType, u16)> {
        self.data.iter().enumerate().map(|(idx, (id, run))| (registry.get_block_type(*id, BlockCoord::from(self.position)+BlockCoord::from(ChunkIdx::from_usize(idx)), commands), *run)).collect()
    }
    pub fn map_to_loaded(&mut self, map: &SavedToLoadedIdMap<BlockId>) {
        for (id, _) in self.data.iter_mut() {
            match map.get(id) {
                Some(loaded_id) => {
                    *id = loaded_id;
                },
                None => {
                    error!("Couldn't map saved block id {:?} to loaded id", id);
                    *id = BlockId(Id::Empty);
                },
            }
        }
    }
}