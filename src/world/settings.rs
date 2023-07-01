use std::path::{Path, PathBuf};

use bevy::prelude::Resource;

use crate::ChunkLoader;

#[derive(Resource)]
pub struct Settings {
    pub init_loader: ChunkLoader,
    pub player_loader: ChunkLoader,
    pub env_path: Box<PathBuf>
}

impl Default for Settings {

    fn default() -> Self {
        let loader = ChunkLoader {
            radius: 6,
            lod_levels: 2,
        };
        Self {
            init_loader: loader.clone(),
            player_loader: loader.clone(),
            env_path: Box::new(Path::new("worlds").join("world"))
        }
    }
}