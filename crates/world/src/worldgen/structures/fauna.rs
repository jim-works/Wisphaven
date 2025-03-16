use crate::{chunk::ChunkIdx, BlockCoord, BlockId};

use super::StructureGenerator;

pub struct FauanaGenerator {
    pub spawn_on: BlockId,
    pub to_spawn: BlockId,
}

impl StructureGenerator for FauanaGenerator {
    fn rarity(&self) -> f32 {
        0.0
    }

    fn generate(
        &self,
        buffer: &mut crate::block_buffer::BlockBuffer<BlockId>,
        _world_seed: u64,
        world_pos: crate::BlockCoord,
        local_pos: ChunkIdx,
        chunk: &crate::chunk::GeneratingChunk,
    ) -> bool {
        if chunk[local_pos.to_usize()] == self.spawn_on {
            buffer.set(
                world_pos + BlockCoord::new(0, 1, 0),
                crate::block_buffer::BlockChange::SetIfEmpty(self.to_spawn),
            );
        }
        true
    }
}
