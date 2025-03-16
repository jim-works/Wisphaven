use bevy::prelude::*;
use bevy::utils::HashMap;

use super::{chunk::*, *};

#[derive(Default)]
pub struct BlockBuffer<T: Clone + Default + PartialEq> {
    pub buf: HashMap<ChunkCoord, ChunkBuffer<T>>,
}

impl BlockBuffer<BlockId> {
    pub fn into_block_type(
        self,
        registry: &BlockRegistry,
        commands: &mut Commands,
    ) -> BlockBuffer<BlockType> {
        let mut result = BlockBuffer::default();
        for (coord, buffer) in self.buf.into_iter() {
            let mut new_buf = ChunkBuffer::default();
            for change in buffer.changes.into_iter() {
                match change.1 {
                    BlockChange::Set(id) => new_buf.changes.push((
                        change.0,
                        BlockChange::Set(registry.get_block_type(id, commands)),
                    )),
                    BlockChange::SetIfEmpty(id) => new_buf.changes.push((
                        change.0,
                        BlockChange::SetIfEmpty(registry.get_block_type(id, commands)),
                    )),
                }
            }
            result.buf.insert(coord, new_buf);
        }
        result
    }
}

impl<T: Clone + Default + PartialEq> BlockBuffer<T> {
    pub fn set(&mut self, coord: BlockCoord, change: BlockChange<T>) {
        let _my_span = info_span!("set_block", name = "set_block").entered();
        let entry = self
            .buf
            .entry(coord.into())
            .or_insert(ChunkBuffer::default());
        entry
            .changes
            .push((ChunkIdx::from(coord).to_usize(), change));
    }
    //moves along the axis with the max distance between a and b repeatedly. not exactly linear but cool
    pub fn place_descending(&mut self, change: BlockChange<T>, a: BlockCoord, b: BlockCoord) {
        let _my_span = info_span!("place_descending", name = "place_descending").entered();
        let mut t = a;
        while t != b {
            self.set(t, change.clone());
            let diff = b - t;
            t += diff.max_component_norm();
        }
    }
    //moves along the axis with the max distance between a and b repeatedly. not exactly linear but cool
    pub fn place_descending_with_block(
        &mut self,
        change: impl Fn(BlockCoord) -> BlockChange<T>,
        a: BlockCoord,
        b: BlockCoord,
    ) {
        let _my_span = info_span!("place_descending", name = "place_descending").entered();
        let mut t = a;
        while t != b {
            self.set(t, change(t));
            let diff = b - t;
            t += diff.max_component_norm();
        }
    }
}

#[derive(Default)]
pub struct ChunkBuffer<T: Clone + Default + PartialEq> {
    changes: Vec<(usize, BlockChange<T>)>,
}

#[derive(Clone)]
pub enum BlockChange<T: Clone> {
    Set(T),
    SetIfEmpty(T),
}

impl<T: Clone + Default + PartialEq> ChunkBuffer<T> {
    pub fn apply_to(self, arr: &mut impl ChunkStorage<T>) {
        for (idx, change) in self.changes {
            match change {
                BlockChange::Set(b) => arr.set_block(idx, b),
                BlockChange::SetIfEmpty(b) => {
                    if std::ops::Index::index(arr, idx) == &T::default() {
                        arr.set_block(idx, b);
                    }
                }
            }
        }
    }
}
