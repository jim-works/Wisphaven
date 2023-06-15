use std::ops::IndexMut;

use bevy::utils::HashMap;
use bevy::prelude::*;

use super::{chunk::*, *};


pub struct BlockBuffer {
    pub buf: HashMap<ChunkCoord, ChunkBuffer>
}

impl BlockBuffer {
    pub fn set(&mut self, coord: BlockCoord, change: BlockChange) {
        let _my_span = info_span!("set_block", name = "set_block").entered();
        let entry = self.buf.entry(coord.into()).or_insert(ChunkBuffer::new());
        entry.changes.push((ChunkIdx::from(coord).to_usize(), change));
    }
    //moves along the axis with the max distance between a and b repeatedly. not exactly linear but cool
    pub fn place_descending(&mut self, change: BlockChange, a: BlockCoord, b: BlockCoord) {
        let _my_span = info_span!("place_descending", name = "place_descending").entered();
        let mut t = a;
        while t != b {
            self.set(t,change.clone());
            let diff = b-t;
            t += diff.max_component_norm();
        }
    }
    pub fn new() -> Self {
        BlockBuffer { buf: HashMap::new() }
    }
}

pub struct ChunkBuffer {
    changes: Vec<(usize, BlockChange)>
}

#[derive(Clone)]
pub enum BlockChange {
    Set(BlockType),
    SetIfEmpty(BlockType)
}

impl ChunkBuffer {
    pub fn new() -> Self {
        Self {changes: Vec::new()}
    }
    pub fn apply_to(self, arr: &mut impl IndexMut<usize, Output=BlockType>) {
        for (idx, change) in self.changes {
            match change {
                BlockChange::Set(b) => arr[idx] = b,
                BlockChange::SetIfEmpty(b) => if matches!(arr[idx], BlockType::Empty) {
                    arr[idx] = b;
                }
            }
        }
    }
}