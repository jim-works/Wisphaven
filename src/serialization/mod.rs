use bevy::prelude::*;
use serde::*;

use crate::world::{chunk::ChunkCoord, LevelLoadState};

pub struct SerializationPlugin;

mod setup;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup::on_level_created.in_set(OnUpdate(LevelLoadState::NotLoaded)));
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChunkSaveFormat {

}

#[derive(Resource)]
pub struct ChunkDB(heed::Database<ChunkCoord, ChunkSaveFormat>);

#[derive(Resource)]
pub struct HeedEnv(heed::Env);