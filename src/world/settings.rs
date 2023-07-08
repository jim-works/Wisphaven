use std::path::{Path, PathBuf};

use bevy::prelude::{Resource, Vec2};

use crate::ChunkLoader;

#[derive(Resource)]
pub struct Settings {
    pub init_loader: ChunkLoader,
    pub player_loader: ChunkLoader,
    pub env_path: Box<PathBuf>,
    pub block_tex_path: Box<PathBuf>,
    pub block_tex_size: Vec2
}

impl Default for Settings {

    fn default() -> Self {
        let loader = ChunkLoader {
            radius: 4,
            lod_levels: 2,
        };
        Self {
            init_loader: loader.clone(),
            player_loader: loader.clone(),
            env_path: Box::new(Path::new("worlds").join("world")),
            block_tex_path: Box::new(Path::new("textures").join("blocks")),
            block_tex_size: Vec2::new(16.0,16.0)
        }
    }
}