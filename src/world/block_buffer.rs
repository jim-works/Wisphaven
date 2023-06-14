use bevy::utils::HashMap;

use crate::util::max_component_norm;

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
    //moves along the axis with the max distance between a and b repeatedly. not exactly linear but cool
    pub fn place_descending(&mut self, block: BlockType, a: BlockCoord, b: BlockCoord) {
        let mut t = a;
        while t != b {
            self.set_block(t,block);
            let diff = b-t;
            t += diff.max_component_norm();
        }
    }
    pub fn new() -> Self {
        BlockBuffer { buf: HashMap::new() }
    }
}

