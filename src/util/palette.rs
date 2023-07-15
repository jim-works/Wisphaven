use std::{
    collections::VecDeque,
    ops::Index,
};

use bevy::prelude::*;

use crate::world::{chunk::{BLOCKS_PER_CHUNK, ChunkBlock, ChunkStorage}, BlockType};

//Assuming that most values in the palette will be small, so V gets cloned instead of referenced in the iterator
pub trait Palette<K, V, I>: Index<usize, Output = V>
where
    I: Iterator<Item = (K, V)>,
    K: Clone,
    V: Clone,
{
    fn index_key(&self, index: usize) -> K;
    fn get_key(&self, value: &V) -> Option<K>;
    fn get_value(&self, key: K) -> Option<&V>;
    fn set(&mut self, index: usize, val: V);
    fn palette_iter(&self) -> I;
}

pub struct PaletteIter<K, V> {
    data: VecDeque<(K, V)>,
}

impl<K, V> Iterator for PaletteIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.data.pop_front()
    }
}

pub trait PaletteMap<K,V,R> {
    fn get_value(&self, key: K) -> Option<&V>;
    fn get_entry_mut(&mut self, key: K) -> Option<&mut (K, V, R)>;
    fn get_entry_mut_value(&mut self, val: &V) -> Option<&mut (K, V, R)>;
    fn get_key(&self, value: &V) -> Option<K>;
}

impl<K: Copy+PartialEq<K>,V: PartialEq<V>> PaletteMap<K,V,u16> for Vec<(K,V,u16)> {
    fn get_value(&self, key: K) -> Option<&V> {
        self
            .iter()
            .find(|(k, _, r)| *k == key && *r > 0)
            .map(|(_, v, _)| v)
    }

    fn get_key(&self, value: &V) -> Option<K> {
        self
            .iter()
            .find(|(_, v, r)| v == value && *r > 0)
            .map(|(k, _, _)| *k)
    }

    fn get_entry_mut(&mut self, key: K) -> Option<&mut (K, V, u16)> {
        self
            .iter_mut()
            .find(|(k, _, r)| *k == key && *r > 0)
    }

    fn get_entry_mut_value(&mut self, val: &V) -> Option<&mut (K, V, u16)> {
        self
            .iter_mut()
            .find(|(_, v, r)| val == v && *r > 0)
    }
}

#[derive(Clone, Debug)]
pub struct BlockPalette<V> {
    pub data: [u16; BLOCKS_PER_CHUNK],
    //I think using a Vec will be faster than hashmap on average, since the number of blocks per chunk will usually be small
    pub palette: Vec<(u16, V, u16)>, //key, value, ref count
}

impl<V> BlockPalette<V> {
    pub fn new(default_val: V) -> Self {
        Self {
            data: [0; BLOCKS_PER_CHUNK],
            palette: vec![(0,default_val,BLOCKS_PER_CHUNK as u16)]
        }
    }
}

impl<V: Clone+ PartialEq<V>> BlockPalette<V> {
    fn get_entry_mut(&mut self, key: u16) -> Option<&mut (u16, V, u16)> {
        self.palette
            .iter_mut()
            .find(|(k, _, r)| *k == key && *r > 0)
    }

    fn get_entry_mut_value(&mut self, val: &V) -> Option<&mut (u16, V, u16)> {
        self.palette
            .iter_mut()
            .find(|(_, v, r)| val == v && *r > 0)
    }
    pub fn iter(&self) -> impl Iterator<Item=&V>{
        self.data.iter().map(|key| self.get_value(*key).unwrap())
    }
}

impl BlockPalette<BlockType> {
    pub fn get_components<T: Component + Clone + PartialEq + Default>(&self, query: &Query<&T>) -> BlockPalette<T>{
        let _span = info_span!("get_components", name = "get_components").entered();
        
        let mut mapped_palette = Vec::with_capacity(self.palette.len());
        for (key,val,r) in self.palette.iter() {
            let block = match val {
                BlockType::Empty => T::default(),
                BlockType::Filled(entity) => query.get(*entity).ok().cloned().unwrap_or_default()
            };
            mapped_palette.push((*key,block,*r));
        }
        BlockPalette { data: self.data.clone(), palette: mapped_palette }
    }
}

impl<V: Clone + PartialEq<V>> Index<usize> for BlockPalette<V> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        self.get_value(self.index_key(index)).unwrap()
    }
}

impl<V: Clone + PartialEq<V>> Palette<u16, V, PaletteIter<u16, V>> for BlockPalette<V> {
    fn index_key(&self, index: usize) -> u16 {
        self.data[index]
    }

    fn get_value(&self, key: u16) -> Option<&V> {
        self.palette
            .iter()
            .find(|(k, _, r)| *k == key && *r > 0)
            .map(|(_, v, _)| v)
    }

    fn get_key(&self, value: &V) -> Option<u16> {
        self.palette
            .iter()
            .find(|(_, v, r)| v == value && *r > 0)
            .map(|(k, _, _)| *k)
    }

    fn set(&mut self, index: usize, val: V) {
        //add or update reference count for the item we're inserting
        let new_key = match self.get_entry_mut_value(&val) {
            Some((k,_,r)) => {
                *r += 1;
                *k
            },
            None => {
                //insert a new key
                //find a vacant spot on the palette if applicable
                let open_spot = self
                    .palette
                    .iter()
                    .enumerate()
                    .find(|(_, (_, _, r))| *r == 0)
                    .map(|(i, _)| i);
                match open_spot {
                    Some(idx) => {
                        self.palette[idx] = (idx as u16, val, 1);
                        idx as u16
                    },
                    None => {
                        let key = self.palette.len() as u16;
                        self.palette.push((key, val, 1));
                        key
                    },
                }
            }
        };
        //decrement old reference count
        let old_ref = self.get_entry_mut(self.index_key(index)).unwrap();
        old_ref.2 -= 1;
        //store new key in data
        self.data[index] = new_key;
    }

    fn palette_iter(&self) -> PaletteIter<u16, V> {
        PaletteIter {
            data: self
                .palette
                .iter()
                .filter(|(_, _, r)| *r > 0)
                .map(|(k, v, _)| (*k, v.clone()))
                .collect(),
        }
    }
}

impl<Block: ChunkBlock> ChunkStorage<Block> for BlockPalette<Block> {
    fn set_block(&mut self, index: usize, val: Block) {
        self.set(index, val);
    }
}