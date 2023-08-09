use std::{collections::VecDeque, ops::Index};

use bevy::prelude::*;

use crate::world::{
    chunk::{
        ChunkBlock, ChunkIdx, ChunkStorage, FatChunkIdx, BLOCKS_PER_CHUNK, BLOCKS_PER_FAT_CHUNK,
        CHUNK_SIZE, CHUNK_SIZE_I8, CHUNK_SIZE_U8,
    },
    BlockType,
};

use super::{Corner, Edge};

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

pub trait PaletteMap<K, V, R> {
    fn get_value(&self, key: K) -> Option<&V>;
    fn get_entry_mut(&mut self, key: K) -> Option<&mut (K, V, R)>;
    fn get_entry_mut_value(&mut self, val: &V) -> Option<&mut (K, V, R)>;
    fn get_key(&self, value: &V) -> Option<K>;
}

impl<K: Copy + PartialEq<K>, V: PartialEq<V>> PaletteMap<K, V, u16> for Vec<(K, V, u16)> {
    fn get_value(&self, key: K) -> Option<&V> {
        self.iter()
            .find(|(k, _, r)| *k == key && *r > 0)
            .map(|(_, v, _)| v)
    }

    fn get_key(&self, value: &V) -> Option<K> {
        self.iter()
            .find(|(_, v, r)| v == value && *r > 0)
            .map(|(k, _, _)| *k)
    }

    fn get_entry_mut(&mut self, key: K) -> Option<&mut (K, V, u16)> {
        self.iter_mut().find(|(k, _, r)| *k == key && *r > 0)
    }

    fn get_entry_mut_value(&mut self, val: &V) -> Option<&mut (K, V, u16)> {
        self.iter_mut().find(|(_, v, r)| val == v && *r > 0)
    }
}

#[derive(Clone, Debug)]
pub struct BlockPalette<V, const SIZE: usize> {
    pub data: [u16; SIZE],
    //I think using a Vec will be faster than hashmap on average, since the number of blocks per chunk will usually be small
    pub palette: Vec<(u16, V, u16)>, //key, value, ref count
}

impl<V, const SIZE: usize> BlockPalette<V, SIZE> {
    pub fn new(default_val: V) -> Self {
        Self {
            data: [0; SIZE],
            palette: vec![(0, default_val, SIZE as u16)],
        }
    }
}

impl<V: Clone + PartialEq<V>, const SIZE: usize> BlockPalette<V, SIZE> {
    fn get_entry_mut(&mut self, key: u16) -> Option<&mut (u16, V, u16)> {
        self.palette
            .iter_mut()
            .find(|(k, _, r)| *k == key && *r > 0)
    }

    fn get_entry_mut_value(&mut self, val: &V) -> Option<&mut (u16, V, u16)> {
        self.palette.iter_mut().find(|(_, v, r)| val == v && *r > 0)
    }
    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.data.iter().map(|key| self.get_value(*key).unwrap())
    }
}

impl<const SIZE: usize> BlockPalette<BlockType, SIZE> {
    pub fn get_components<T: Component + Clone + PartialEq + Default>(
        &self,
        query: &Query<&T>,
    ) -> BlockPalette<T, SIZE> {
        let _span = info_span!("get_components", name = "get_components").entered();
        BlockPalette {
            data: self.data.clone(),
            palette: self.map_palette(query),
        }
    }
    pub fn get_component<T: Component + Clone + PartialEq + Default>(
        &self,
        idx: usize,
        query: &Query<&T>,
    ) -> T {
        self.get_value(self.data[idx])
            .map(|block| match block {
                BlockType::Empty => T::default(),
                BlockType::Filled(entity) => query.get(*entity).ok().cloned().unwrap_or_default(),
            })
            .unwrap_or_default()
    }
    pub fn map_palette<T: Component + Clone + PartialEq + Default>(
        &self,
        query: &Query<&T>,
    ) -> Vec<(u16, T, u16)> {
        let _span = info_span!("map_palette", name = "map_palette").entered();

        let mut mapped_palette = Vec::with_capacity(self.palette.len());
        for (key, val, r) in self.palette.iter() {
            let block = match val {
                BlockType::Empty => T::default(),
                BlockType::Filled(entity) => query.get(*entity).ok().cloned().unwrap_or_default(),
            };
            mapped_palette.push((*key, block, *r));
        }
        mapped_palette
    }
}

impl BlockPalette<BlockType, BLOCKS_PER_CHUNK> {
    //gets all components using the query, and creates palette for a fat chunk.
    //there is one block taken from the neighboring chunks in each direction, so it has size (CHUNK_SIZE+2)^3
    //if any of the neighbors is None, uses default value of component
    pub fn create_fat_palette<T: Component + Clone + PartialEq + Default>(
        &self,
        query: &Query<&T>,
        //full chunks of CHUNK_SIZExCHUNK_SIZE, array indexed by crate::util::Direction
        face_neighbors: [Option<impl Index<usize, Output = T>>; 6],
        //strips of 1xCHUNK_SIZE, array indexed by crate::util::Edge
        edge_neighbors: [Option<[T; CHUNK_SIZE]>; 12],
        //single blocks for the corners, array indexed by crate::util::Corner
        corner_neighbors: [Option<T>; 8],
    ) -> BlockPalette<T, BLOCKS_PER_FAT_CHUNK> {
        let _span = info_span!("create_fat_palette", name = "create_fat_palette").entered();
        //start by copying over all my information
        let mut fat_palette: BlockPalette<T, { BLOCKS_PER_FAT_CHUNK }> =
            BlockPalette::new(T::default());
        //let mut data: [u16; BLOCKS_PER_FAT_CHUNK] = [0; BLOCKS_PER_FAT_CHUNK];
        for x in 0..CHUNK_SIZE_I8 {
            for y in 0..CHUNK_SIZE_I8 {
                for z in 0..CHUNK_SIZE_I8 {
                    fat_palette.set(
                        Into::<usize>::into(FatChunkIdx::new(x, y, z)),
                        self.get_component(
                            Into::<usize>::into(ChunkIdx::new(x as u8, y as u8, z as u8)),
                            query,
                        ),
                    )
                }
            }
        }
        //neg x face
        fat_palette.fat_add_face(
            &face_neighbors[crate::util::Direction::NegX.to_idx()],
            |y, z| FatChunkIdx::new(-1, y, z).into(),
            |y, z| Into::<usize>::into(ChunkIdx::new(CHUNK_SIZE_U8 - 1, y as u8, z as u8)),
        );
        //pos x face
        fat_palette.fat_add_face(
            &face_neighbors[crate::util::Direction::PosX.to_idx()],
            |y, z| FatChunkIdx::new(CHUNK_SIZE_I8, y, z).into(),
            |y, z| Into::<usize>::into(ChunkIdx::new(0, y as u8, z as u8)),
        );
        //neg y face
        fat_palette.fat_add_face(
            &face_neighbors[crate::util::Direction::NegY.to_idx()],
            |x, z| FatChunkIdx::new(x, -1, z).into(),
            |x, z| Into::<usize>::into(ChunkIdx::new(x as u8, CHUNK_SIZE_U8 - 1, z as u8)),
        );
        //pos y face
        fat_palette.fat_add_face(
            &face_neighbors[crate::util::Direction::PosY.to_idx()],
            |x, z| FatChunkIdx::new(x, CHUNK_SIZE_I8, z).into(),
            |x, z| Into::<usize>::into(ChunkIdx::new(x as u8, 0, z as u8)),
        );        
        //neg z face
        fat_palette.fat_add_face(
            &face_neighbors[crate::util::Direction::NegZ.to_idx()],
            |x, y| FatChunkIdx::new(x, y, -1).into(),
            |x, y| Into::<usize>::into(ChunkIdx::new(x as u8, y as u8, CHUNK_SIZE_U8 - 1)),
        );
        //pos z face
        fat_palette.fat_add_face(
            &face_neighbors[crate::util::Direction::PosZ.to_idx()],
            |x, y| FatChunkIdx::new(x, y, CHUNK_SIZE_I8).into(),
            |x, y| Into::<usize>::into(ChunkIdx::new(x as u8, y as u8, CHUNK_SIZE_U8 - 1)),
        );

        //corners
        for corner_label in Corner::iter() {
            fat_palette.fat_add_corner(
                corner_neighbors[corner_label as usize].clone(),
                corner_label,
            );
        }
        //edges
        for edge_label in Edge::iter() {
            fat_palette.fat_add_edge(edge_neighbors[edge_label as usize].clone(), edge_label);
        }
        fat_palette
    }
}

impl<T: Component + Clone + PartialEq + Default> BlockPalette<T, BLOCKS_PER_FAT_CHUNK> {
    fn fat_add_face(
        &mut self,
        neighbor: &Option<impl Index<usize, Output = T>>,
        self_idx: impl Fn(i8, i8) -> usize,
        neighbor_idx: impl Fn(i8, i8) -> usize,
    ) {
        match neighbor {
            Some(face) => {
                for y in 0..CHUNK_SIZE_I8 {
                    for z in 0..CHUNK_SIZE_I8 {
                        self.set(self_idx(y, z), face[neighbor_idx(y, z)].clone());
                    }
                }
            }
            None => {
                for y in 0..CHUNK_SIZE_I8 {
                    for z in 0..CHUNK_SIZE_I8 {
                        self.set(self_idx(y, z), T::default());
                    }
                }
            }
        }
    }
    fn fat_add_corner(&mut self, corner: Option<T>, corner_label: Corner) {
        self.set(
            match corner_label {
                Corner::NXNYNZ => FatChunkIdx::new(-1, -1, -1),
                Corner::NXNYPZ => FatChunkIdx::new(-1, -1, CHUNK_SIZE_I8),
                Corner::NXPYNZ => FatChunkIdx::new(-1, CHUNK_SIZE_I8, -1),
                Corner::NXPYPZ => FatChunkIdx::new(-1, CHUNK_SIZE_I8, CHUNK_SIZE_I8),
                Corner::PXNYNZ => FatChunkIdx::new(CHUNK_SIZE_I8, -1, -1),
                Corner::PXNYPZ => FatChunkIdx::new(CHUNK_SIZE_I8, -1, CHUNK_SIZE_I8),
                Corner::PXPYNZ => FatChunkIdx::new(CHUNK_SIZE_I8, CHUNK_SIZE_I8, -1),
                Corner::PXPYPZ => FatChunkIdx::new(CHUNK_SIZE_I8, CHUNK_SIZE_I8, CHUNK_SIZE_I8),
            }
            .into(),
            corner.unwrap_or_default(),
        );
    }
    fn fat_add_edge(&mut self, edge: Option<[T; CHUNK_SIZE]>, edge_label: Edge) {
        match edge {
            Some(edge) => {
                for (i, t) in edge.into_iter().enumerate() {
                    self.set(
                        //we use i+1 to move one extra unit in the direction of the edge, so that it doesn't start in the corner
                        FatChunkIdx::new(
                            (edge_label.fat_origin().x as i32
                                + (i + 1) as i32 * edge_label.direction().x)
                                as i8,
                            (edge_label.fat_origin().y as i32
                                + (i + 1) as i32 * edge_label.direction().y)
                                as i8,
                            (edge_label.fat_origin().z as i32
                                + (i + 1) as i32 * edge_label.direction().z)
                                as i8,
                        )
                        .into(),
                        t,
                    )
                }
            }
            None => {
                for i in 0..CHUNK_SIZE {
                    self.set(
                        //we use i+1 to move one extra unit in the direction of the edge, so that it doesn't start in the corner
                        FatChunkIdx::new(
                            (edge_label.fat_origin().x as i32
                                + (i + 1) as i32 * edge_label.direction().x)
                                as i8,
                            (edge_label.fat_origin().y as i32
                                + (i + 1) as i32 * edge_label.direction().y)
                                as i8,
                            (edge_label.fat_origin().z as i32
                                + (i + 1) as i32 * edge_label.direction().z)
                                as i8,
                        )
                        .into(),
                        T::default(),
                    )
                }
            }
        }
    }
}

impl<V: Clone + PartialEq<V>, const SIZE: usize> Index<usize> for BlockPalette<V, SIZE> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        self.get_value(self.index_key(index)).unwrap()
    }
}

impl<V: Clone + PartialEq<V>, const SIZE: usize> Palette<u16, V, PaletteIter<u16, V>>
    for BlockPalette<V, SIZE>
{
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
            Some((k, _, r)) => {
                *r += 1;
                *k
            }
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
                    }
                    None => {
                        let key = self.palette.len() as u16;
                        self.palette.push((key, val, 1));
                        key
                    }
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

impl<Block: ChunkBlock, const SIZE: usize> ChunkStorage<Block> for BlockPalette<Block, SIZE> {
    fn set_block(&mut self, index: usize, val: Block) {
        self.set(index, val);
    }
}
