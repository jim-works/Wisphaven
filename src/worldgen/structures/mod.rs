use std::sync::Arc;

use bracket_noise::prelude::FastNoise;
use bevy::prelude::*;

use crate::{world::{chunk::*, BlockBuffer, BlockCoord, BlockId}, util::get_next_prng};

use super::{DecorationSettings, pipeline::ColumnBiomes, biomes::UsedBiomeMap};

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
    //returns false if chunk is outside of the structure's bounds.
    fn generate(&self, buffer: &mut BlockBuffer<BlockId>, world_pos: BlockCoord, local_pos: ChunkIdx, chunk: &GeneratingChunk) -> bool;
}

pub trait LargeStructureGenerator: StructureGenerator {
    fn setup(&mut self, world_pos: BlockCoord);
}

pub fn gen_structures(chunk: &mut GeneratingChunk, biomes: ColumnBiomes<CHUNK_SIZE>, biome_map: &UsedBiomeMap, settings: Arc<StructureGenerationSettings>) -> BlockBuffer<BlockId> {
    let _my_span = info_span!("gen_small_structures", name = "gen_small_structures").entered();
    let mut buf = BlockBuffer::new();
    let biome = biome_map.get(biomes.0[0][0]);
    if let Some(gen) = &biome.fallback_generator {
        gen.generate(&mut buf, chunk.position.into(), ChunkIdx::new(0,0,0), chunk);
    }
    
    buf
}