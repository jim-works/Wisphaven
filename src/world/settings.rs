use bevy::{prelude::*, math::UVec2};

use crate::ChunkLoader;

use super::chunk::ChunkCoord;

#[derive(Resource)]
pub struct Settings {
    pub init_loader: ChunkLoader,
    pub player_loader: ChunkLoader,
    pub env_path: &'static str,
    pub block_tex_path: &'static str,
    pub block_type_path: &'static str,
    pub item_tex_path: &'static str,
    pub item_type_path: &'static str,
    pub recipe_path: &'static str,
    pub block_tex_size: UVec2
}

impl Default for Settings {

    fn default() -> Self {
        let loader = ChunkLoader {
            radius: ChunkCoord::new(12,8,12),
            lod_levels: 0,
            mesh: true
        };
        Self {
            init_loader: loader.clone(),
            player_loader: ChunkLoader {
                radius: ChunkCoord::new(12, 8,12),
                ..loader.clone()
            },
            env_path: "worlds/world",
            //prefixed with "assets/"
            block_tex_path: "textures/blocks",
            //prefixed with "assets/"
            block_type_path: "blocks",
            //prefixed with "assets/"
            item_tex_path: "textures/items",
            //prefixed with "assets/"
            item_type_path: "items",
            //prefixed with "assets/"
            recipe_path: "recipes",
            block_tex_size: UVec2::new(16,16)
        }
    }
}