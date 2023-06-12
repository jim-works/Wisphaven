use bevy::utils::HashMap;

use super::{chunk::*, *};


pub struct BlockBuffer {
    pub buf: HashMap<ChunkCoord, Box<[BlockType; BLOCKS_PER_CHUNK]>>
}

impl BlockBuffer {
    pub fn set_block(&mut self, coord: BlockCoord, block: BlockType) {
        let entry = self.buf.entry(coord.into()).or_insert(Box::new([BlockType::Empty; BLOCKS_PER_CHUNK]));
        entry[ChunkIdx::from(coord).to_usize()] = block;
    }
    pub fn set_if_empty(&mut self, coord: BlockCoord, block: BlockType) {
        let entry = self.buf.entry(coord.into()).or_insert(Box::new([BlockType::Empty; BLOCKS_PER_CHUNK]));
        let coord = ChunkIdx::from(coord).to_usize();
        if matches!(entry[coord], BlockType::Empty) {
            entry[coord] = block;
        }
    }
    pub fn new() -> Self {
        BlockBuffer { buf: HashMap::new() }
    }
}

