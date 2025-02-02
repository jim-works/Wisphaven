use std::path::PathBuf;

use bevy::{prelude::*, utils::HashMap};

use interfaces::scheduling::{LevelSystemSet, NetworkType};
use world::chunk::ChunkCoord;

pub struct SerializationPlugin;

pub mod db;
mod loading;
pub mod queries;
mod saving;
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
                    saving::do_saving,
                    saving::save_all,
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
pub struct ItemTextureMap(pub HashMap<PathBuf, Handle<Image>>);

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
