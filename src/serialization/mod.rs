use bevy::prelude::*;
use heed::types::{SerdeBincode, OwnedType, UnalignedSlice, ByteSlice};

use crate::world::{chunk::{ChunkCoord, ArrayChunk}, LevelLoadState, BlockType, LevelSystemSet};

pub struct SerializationPlugin;

mod setup;
mod save;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup::on_level_created.in_set(OnUpdate(LevelLoadState::NotLoaded)))
            .add_system(save::do_saving.in_set(LevelSystemSet::Main))
        ;
    }
}

#[derive(Component)]
pub struct NeedsSaving;

//run length encoded format for chunks
//TODO: figure out how to do entities
pub struct ChunkSaveFormat {
    pub position: ChunkCoord,
    pub data: Vec<(BlockType, u16)>
}

impl From<&ArrayChunk> for ChunkSaveFormat {
    fn from(value: &ArrayChunk) -> Self {
        let mut data = Vec::new();
        let mut run = 1;
        let mut curr_block_opt = None;
        for block in value.blocks.into_iter() {
            match curr_block_opt {
                None => {curr_block_opt = Some(block)},
                Some(curr_block) => {
                    if curr_block == block {
                        run += 1;
                    } else {
                        data.push((curr_block, run));
                        curr_block_opt = Some(block);
                        run = 1;
                    }
                }
            }
        }
        return Self {
            position: value.position,
            data
        }
    }
}

impl ChunkSaveFormat {
    fn to_chunk(self, chunk_entity: Entity) -> ArrayChunk {
        let mut curr_idx = 0;
        let mut chunk = ArrayChunk::new(self.position, chunk_entity);
        for (block, length) in self.data.into_iter() {
            for idx in curr_idx..curr_idx+length as usize {
                chunk.blocks[idx] = block;
            }
        };
        chunk
    }
}

#[derive(Resource)]
pub struct ChunkDB(heed::Database<SerdeBincode<ChunkCoord>, ByteSlice>);

#[derive(Resource)]
pub struct HeedEnv(heed::Env);