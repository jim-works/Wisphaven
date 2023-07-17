use std::sync::Arc;

use bracket_noise::prelude::FastNoise;
use bevy::prelude::*;

use crate::{world::{chunk::{ChunkIdx, CHUNK_SIZE_F32, CHUNK_SIZE_U64, GeneratingChunk}, BlockBuffer, BlockCoord, BlockId}, util::get_next_prng};

use super::DecorationSettings;

pub mod trees;

#[derive(Resource)]
pub struct StructureResources {
    pub settings: Arc<StructureGenerationSettings>
}

pub struct StructureGenerationSettings {
    pub rolls_per_chunk: i32,
    pub structures: Vec<Box<dyn StructureGenerator + Sync + Send>>,
    pub placement_noise: FastNoise
}

pub trait StructureGenerator {
    fn rarity(&self) -> f32;
    fn generate(&self, buffer: &mut BlockBuffer<BlockId>, world_pos: BlockCoord, local_pos: ChunkIdx, chunk: &GeneratingChunk);
}

pub fn gen_small_structures(chunk: &mut GeneratingChunk, settings: Arc<StructureGenerationSettings>) -> BlockBuffer<BlockId> {
    let _my_span = info_span!("gen_small_structures", name = "gen_small_structures").entered();
    let mut buf = BlockBuffer::new();
    for roll in 0..settings.rolls_per_chunk {
        //rescale from [-1,1] to [0,CHUNK_SIZE]
        let t = (settings.placement_noise.get_noise3d((chunk.position.x+roll) as f32, (chunk.position.y+roll) as f32, (chunk.position.z+roll) as f32)+1.0)*CHUNK_SIZE_F32/2.0;
        let x = get_next_prng(t as u64);
        let y = get_next_prng(x);
        let z = get_next_prng(y);
        for structure in &settings.structures {
            let pos = ChunkIdx::new((x%CHUNK_SIZE_U64) as u8, (y%CHUNK_SIZE_U64) as u8, (z%CHUNK_SIZE_U64) as u8);
            structure.generate(&mut buf, BlockCoord::from(chunk.position)+BlockCoord::new(pos.x as i32, pos.y as i32, pos.z as i32), pos, &chunk)
        }
    }
    
    buf
}