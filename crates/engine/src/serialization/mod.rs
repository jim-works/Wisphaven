use std::path::PathBuf;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use bevy::{prelude::*, utils::HashMap};

use crate::{
    net::NetworkType,
    world::{
        chunk::{ArrayChunk, ChunkCoord, ChunkTrait, BLOCKS_PER_CHUNK},
        util::BlockPalette,
        BlockId, BlockRegistry, BlockType, Id, LevelSystemSet,
    },
};

pub struct SerializationPlugin;

pub mod db;
mod loading;
pub mod queries;
mod save;
mod setup;
pub mod state;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((state::SerializationStatePlugin, setup::SetupPlugin))
            //load/save loop
            //do not do if a client, it will recieve its information from the server
            .add_systems(
                Update,
                (
                    loading::load_chunk_terrain,
                    loading::queue_terrain_loading,
                    db::tick_db,
                    save::do_saving,
                    save::save_all,
                )
                    .in_set(LevelSystemSet::AfterLoadingAndMain)
                    .run_if(not(in_state(NetworkType::Client))),
            )
            .add_event::<SaveChunkEvent>()
            .add_event::<db::DataFromDBEvent>()
            .insert_resource(SaveTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
            .init_resource::<LevelCreationInput>();
    }
}

#[derive(Resource)]
pub struct SavedLevels(pub Vec<SavedLevelInfo>);

pub struct SavedLevelInfo {
    pub name: &'static str,
    pub modified_time: std::time::SystemTime,
}

#[derive(Resource)]
pub struct LevelCreationInput {
    pub name: &'static str,
    pub seed: Option<u64>,
}

impl Default for LevelCreationInput {
    fn default() -> Self {
        Self {
            name: "level",
            seed: None,
        }
    }
}

#[derive(Resource)]
pub struct BlockTextureMap(pub HashMap<PathBuf, u32>);

#[derive(Resource)]
pub struct ItemTextureMap(pub HashMap<PathBuf, Handle<Image>>);

#[derive(Resource, Default)]
pub struct SavedToLoadedIdMap<T: Into<Id> + Clone + From<Id> + std::hash::Hash + Eq + PartialEq> {
    pub map: HashMap<T, T>,
    pub max_key_id: u32,
}

impl<T: Into<Id> + From<Id> + Clone + std::hash::Hash + Eq + PartialEq> SavedToLoadedIdMap<T> {
    pub fn insert(&mut self, key: T, val: T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => {}
            Id::Basic(id) | Id::Dynamic(id) => self.max_key_id = self.max_key_id.max(id),
        }
        self.map.insert(key, val)
    }
    pub fn get(&self, key: &T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => Some(T::from(Id::Empty)),
            _ => self.map.get(key).cloned(),
        }
    }
}

#[derive(Resource, Default)]
pub struct LoadedToSavedIdMap<T: Into<Id> + Clone + From<Id> + std::hash::Hash + Eq + PartialEq> {
    pub map: HashMap<T, T>,
    pub max_key_id: u32,
}

impl<T: Into<Id> + From<Id> + std::hash::Hash + Clone + Eq + PartialEq> LoadedToSavedIdMap<T> {
    pub fn insert(&mut self, key: T, val: T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => {}
            Id::Basic(id) | Id::Dynamic(id) => self.max_key_id = self.max_key_id.max(id),
        }
        self.map.insert(key, val)
    }
    pub fn get(&self, key: &T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => Some(T::from(Id::Empty)),
            _ => self.map.get(key).cloned(),
        }
    }
}

#[derive(Component)]
pub struct NeedsSaving;

#[derive(Component)]
pub struct NeedsLoading;

#[derive(Component, Copy, Clone)]
pub struct LoadingBlocks;

#[derive(Component, Clone, Copy)]
pub struct LoadingItems;

#[derive(Component, Clone, Copy)]
pub struct RecipesScene;

#[derive(Resource)]
pub struct SaveTimer(Timer);

#[derive(Event)]
pub struct SaveChunkEvent(ChunkCoord);

//run length encoded format for chunks
//TODO: figure out how to do entities
#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub fn ids_only(
        value: (ChunkCoord, &[BlockType; BLOCKS_PER_CHUNK]),
        query: &Query<&BlockId>,
        map: &LoadedToSavedIdMap<BlockId>,
    ) -> Self {
        let data = value
            .1
            .iter()
            .dedup_with_count()
            .map(|(run, block)| {
                (
                    match block {
                        BlockType::Empty => BlockId(Id::Empty),
                        BlockType::Filled(entity) => map
                            .get(query.get(*entity).unwrap_or(&BlockId(Id::Empty)))
                            .unwrap(),
                    },
                    run as u16,
                )
            })
            .collect();
        Self {
            position: value.0,
            data,
        }
    }
    //creates a save format by extracting the ids from the block array using the provided query
    //will replace with the empty block if the entities in the block array do not have a BlockId component
    pub fn palette_ids_only(
        value: (ChunkCoord, &BlockPalette<BlockType, BLOCKS_PER_CHUNK>),
        query: &Query<&BlockId>,
        map: &LoadedToSavedIdMap<BlockId>,
    ) -> Self {
        let data = value
            .1
            .get_components(query)
            .iter()
            .dedup_with_count()
            .map(|(run, block)| (map.get(block).unwrap(), run as u16))
            .collect();
        Self {
            position: value.0,
            data,
        }
    }
    //creates a save format by extracting the ids from the block array using the provided query
    //will replace with the empty block if the entities in the block array do not have a BlockId component
    pub fn palette_ids_only_no_map(
        value: (ChunkCoord, &BlockPalette<BlockType, BLOCKS_PER_CHUNK>),
        query: &Query<&BlockId>,
    ) -> Self {
        let data = value
            .1
            .get_components(query)
            .iter()
            .dedup_with_count()
            .map(|(run, block)| (*block, run as u16))
            .collect();
        Self {
            position: value.0,
            data,
        }
    }
    pub fn into_chunk(
        self,
        chunk_entity: Entity,
        registry: &BlockRegistry,
        commands: &mut Commands,
    ) -> ArrayChunk {
        let mut curr_idx = 0;
        let mut chunk = ArrayChunk::new(self.position, chunk_entity);
        for (block, length) in self.data.into_iter() {
            for idx in curr_idx..curr_idx + length as usize {
                chunk.set_block(idx, registry.get_block_type(block, commands));
            }
            curr_idx += length as usize;
        }
        chunk
    }
    pub fn into_buffer(
        self,
        registry: &BlockRegistry,
        commands: &mut Commands,
    ) -> Vec<(BlockType, u16)> {
        self.data
            .iter()
            .map(|(id, run)| (registry.get_block_type(*id, commands), *run))
            .collect()
    }
    pub fn map_to_loaded(&mut self, map: &SavedToLoadedIdMap<BlockId>) {
        for (id, _) in self.data.iter_mut() {
            match map.get(id) {
                Some(loaded_id) => {
                    *id = loaded_id;
                }
                None => {
                    error!("Couldn't map saved block id {:?} to loaded id", id);
                    *id = BlockId(Id::Empty);
                }
            }
        }
    }
}
