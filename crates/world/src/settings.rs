use bevy::{math::UVec2, prelude::*};

use crate::chunk_loading::ChunkLoader;

use super::chunk::ChunkCoord;

#[derive(Resource)]
pub struct Settings {
    pub init_loader: ChunkLoader,
    pub player_loader: ChunkLoader,
    pub anchor_loader: ChunkLoader,
    pub env_path: &'static str,
    pub block_tex_path: &'static str,
    pub block_type_path: &'static str,
    pub item_tex_path: &'static str,
    pub item_type_path: &'static str,
    pub recipe_path: &'static str,
    pub block_tex_size: UVec2,
    pub mouse_sensitivity: f32,
}

impl Default for Settings {
    fn default() -> Self {
        let loader = ChunkLoader {
            radius: ChunkCoord::new(12, 8, 12),
            lod_levels: 0,
            mesh: true,
        };
        Self {
            init_loader: loader.clone(),
            player_loader: ChunkLoader {
                radius: ChunkCoord::new(2, 2, 2),
                ..loader.clone()
            },
            anchor_loader: ChunkLoader {
                mesh: false,
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
            block_tex_size: UVec2::new(16, 16),
            mouse_sensitivity: 0.005,
        }
    }
}

#[derive(Resource)]
pub struct GraphicsSettings {
    pub particle_animation_distance: f32,
    pub hand_hit_animation_duration: f32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            particle_animation_distance: 128.0,
            hand_hit_animation_duration: 0.1,
        }
    }
}
