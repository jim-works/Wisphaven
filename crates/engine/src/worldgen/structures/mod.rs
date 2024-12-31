use bevy::prelude::*;

use crate::world::{chunk::*, BlockBuffer, BlockCoord, BlockId};

use super::{biomes::UsedBiomeMap, pipeline::ColumnBiomes};

pub mod fauna;
pub mod trees;

pub trait StructureGenerator {
    fn rarity(&self) -> f32;
    //returns false if chunk is outside of the structure's bounds.
    fn generate(
        &self,
        buffer: &mut BlockBuffer<BlockId>,
        world_seed: u64,
        world_pos: BlockCoord,
        local_pos: ChunkIdx,
        chunk: &GeneratingChunk,
    ) -> bool;
}

pub trait LargeStructureGenerator: StructureGenerator {
    fn setup(&mut self, world_pos: BlockCoord);
}

pub fn gen_structures(
    chunk: &mut GeneratingChunk,
    seed: u64,
    biomes: ColumnBiomes<CHUNK_SIZE>,
    biome_map: &UsedBiomeMap,
) -> BlockBuffer<BlockId> {
    let _my_span = info_span!("gen_small_structures", name = "gen_small_structures").entered();
    let mut buf = BlockBuffer::default();
    let biome = biome_map.get(biomes.0[0][0]);
    if let Some(gen) = &biome.fallback_generator {
        gen.generate(
            &mut buf,
            seed,
            chunk.position.into(),
            ChunkIdx::new(0, 0, 0),
            chunk,
        );
    }

    buf
}
