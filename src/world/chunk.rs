use std::{ops::{Index, IndexMut, Add}, marker::PhantomData};

use bevy::prelude::*;
use serde::{Serialize, Deserialize};

use crate::util::Direction;

use super::{BlockType, BlockCoord, BlockId, BlockRegistry, BlockMesh};

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_SIZE_U8: u8 = CHUNK_SIZE as u8;
pub const CHUNK_SIZE_U64: u64 = CHUNK_SIZE as u64;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LODLevel {pub level: u8}

pub type ArrayChunk = Chunk<[BlockType; BLOCKS_PER_CHUNK], BlockType>;
pub type LODChunk = ArrayChunk;
pub type GeneratingChunk = Chunk<[BlockId; BLOCKS_PER_CHUNK], BlockId>;
pub type GeneratingLODChunk = GeneratingChunk;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32
}

impl ChunkCoord {
    pub fn new (x: i32, y: i32, z: i32) -> ChunkCoord {
        ChunkCoord { x, y, z }
    }
    pub fn to_vec3(self) -> Vec3 {
        Vec3::new((self.x*CHUNK_SIZE_I32) as f32,(self.y*CHUNK_SIZE_I32) as f32,(self.z*CHUNK_SIZE_I32) as f32)
    }
    pub fn offset(self, dir: Direction) -> ChunkCoord {
        match dir {
            Direction::PosX => ChunkCoord::new(self.x+1,self.y,self.z),
            Direction::PosY => ChunkCoord::new(self.x,self.y+1,self.z),
            Direction::PosZ => ChunkCoord::new(self.x,self.y,self.z+1),
            Direction::NegX => ChunkCoord::new(self.x-1,self.y,self.z),
            Direction::NegY => ChunkCoord::new(self.x,self.y-1,self.z),
            Direction::NegZ => ChunkCoord::new(self.x,self.y,self.z-1),
        }
    }
    pub fn to_next_lod(self) -> ChunkCoord {
        ChunkCoord::new(self.x/2,self.y/2,self.z/2)
    }
}

impl Add<ChunkCoord> for ChunkCoord {
    type Output = ChunkCoord;

    fn add(self, rhs: ChunkCoord) -> Self::Output {
        ChunkCoord::new(self.x+rhs.x,self.y+rhs.y,self.z+rhs.z)
    }
}

impl From<Vec3> for ChunkCoord {
    fn from(v: Vec3) -> Self {
        ChunkCoord::new((v.x/CHUNK_SIZE_F32).floor() as i32,(v.y/CHUNK_SIZE_F32).floor() as i32,(v.z/CHUNK_SIZE_F32).floor() as i32)
    }
}

impl From<BlockCoord> for ChunkCoord {
    fn from(v: BlockCoord) -> Self {
        ChunkCoord::new(v.x.div_euclid(CHUNK_SIZE_I32),v.y.div_euclid(CHUNK_SIZE_I32),v.z.div_euclid(CHUNK_SIZE_I32))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkIdx {
    pub x: u8,
    pub y: u8,
    pub z: u8
}

impl ChunkIdx {
    pub fn new (x: u8, y: u8, z: u8) -> ChunkIdx {
        ChunkIdx { x, y, z }
    }
    pub fn from_usize (i: usize) -> ChunkIdx {
        let x = i/(CHUNK_SIZE*CHUNK_SIZE);
        let y = (i-x*CHUNK_SIZE*CHUNK_SIZE)/CHUNK_SIZE;
        let z = i-x*CHUNK_SIZE*CHUNK_SIZE-y*CHUNK_SIZE;
        ChunkIdx { x: x as u8, y: y as u8, z: z as u8 }
    }
    //on the corner of the block, block extends in positive directions
    pub fn to_vec3(self) -> Vec3 {
        Vec3::new(self.x as f32,self.y as f32,self.z as f32)
    }
    pub fn get_block_center(self) -> Vec3 {
        Vec3::new(self.x as f32+0.5,self.y as f32+0.5,self.z as f32+0.5)
    }
    pub fn to_usize(self) -> usize {
        (self.x as usize)*CHUNK_SIZE*CHUNK_SIZE+(self.y as usize)*CHUNK_SIZE+(self.z as usize)
    }
    
}

impl From<BlockCoord> for ChunkIdx {
    fn from(v: BlockCoord) -> Self {
        ChunkIdx::new(v.x.rem_euclid(CHUNK_SIZE_I32) as u8,v.y.rem_euclid(CHUNK_SIZE_I32) as u8,v.z.rem_euclid(CHUNK_SIZE_I32) as u8)
    }
    
}

impl Add<ChunkIdx> for ChunkIdx {
    type Output = Self;

    fn add(self, rhs: ChunkIdx) -> Self::Output {
        ChunkIdx::new(self.x+rhs.x,self.y+rhs.y,self.z+rhs.z)
    }
}

#[derive(Clone, Debug)]
pub enum ChunkType {
    Ungenerated(Entity),
    Full(ArrayChunk)
}

#[derive(Clone, Debug)]
pub enum LODChunkType {
    //entity, level
    Ungenerated(Entity, u8),
    Full(LODChunk)
}

pub trait ChunkStorage<Block>: Index<usize, Output=Block> + IndexMut<usize, Output=Block> {}
impl<T, Block> ChunkStorage<Block> for T where T: Index<usize, Output=Block> + IndexMut<usize, Output=Block> {}

pub trait ChunkBlock: Clone + Send + Sync + PartialEq {}
impl<T> ChunkBlock for T where T: Clone + Send + Sync + PartialEq {}

#[derive(Clone, Debug)]
pub struct Chunk<Storage, Block> where Storage: ChunkStorage<Block>, Block: ChunkBlock {
    pub blocks: Box<Storage>,
    pub position: ChunkCoord,
    pub entity: Entity,
    //lod level, scale of chunk is 2^level
    pub level: u8,
    //not sure how to get around this
    _data: PhantomData<Block>
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> Index<ChunkIdx> for Chunk<Storage, Block> {
    type Output = Block;
    fn index(&self, index: ChunkIdx) -> &Block {
        &self.blocks[index.to_usize()]
    }
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> IndexMut<ChunkIdx> for Chunk<Storage, Block> {
    fn index_mut(&mut self, index: ChunkIdx) -> &mut Block {
        &mut self.blocks[index.to_usize()]
    }
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> Index<usize> for Chunk<Storage, Block> {
    type Output = Block;
    fn index(&self, index: usize) -> &Block {
        &self.blocks[index]
    }
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> IndexMut<usize> for Chunk<Storage, Block> {
    fn index_mut(&mut self, index: usize) -> &mut Block {
        &mut self.blocks[index]
    }
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> Chunk<Storage,Block> {
    pub fn scale(&self) -> i32 {
        LODChunk::level_to_scale(self.level)
    }

    pub fn level_to_scale(level: u8) -> i32 {
        1<<level
    }

    pub fn get_block_pos(&self, pos: ChunkIdx) -> Vec3 {
        let scale = self.scale();
        Vec3::new((self.position.x*CHUNK_SIZE_I32*scale+(pos.x as i32)*scale) as f32, (self.position.y*CHUNK_SIZE_I32*scale+(pos.y as i32)*scale) as f32, (self.position.z*CHUNK_SIZE_I32*scale+(pos.z as i32)*scale) as f32)
    }
}

impl<'a, Storage: ChunkStorage<BlockType>> Chunk<Storage, BlockType> {
    pub fn get_components<T: Component + Clone + PartialEq + Default>(&self, block_iter: impl Iterator<Item = &'a BlockType>, query: &Query<&T>) -> Chunk<Vec<T>, T>{
        let data = block_iter.map(|b| match b {
            BlockType::Empty => T::default(),
            BlockType::Filled(entity) => query.get(*entity).unwrap_or(&T::default()).to_owned(),
        }).collect();
        Chunk::<Vec<T>, T> {blocks: Box::new(data), position: self.position, entity: self.entity, level: self.level, _data: PhantomData }
    }
}
impl ArrayChunk {
    pub fn new(position: ChunkCoord, entity: Entity) -> ArrayChunk {
        Chunk {
            blocks: Box::new([BlockType::Empty; BLOCKS_PER_CHUNK]),
            entity,
            position,
            level: 1,
            _data: PhantomData,
        }
    }
}

impl GeneratingChunk {
    pub fn new(position: ChunkCoord, entity: Entity) -> GeneratingChunk {
        Chunk {
            blocks: Box::new([BlockId::default(); BLOCKS_PER_CHUNK]),
            entity,
            position,
            level: 1,
            _data: PhantomData
        }
    }

    pub fn to_array_chunk(self, registry: &BlockRegistry, commands: &mut Commands) -> ArrayChunk {
        let mut result = ArrayChunk::new(self.position, self.entity);
        for (i, block) in self.blocks.into_iter().enumerate() {
            result[i] = match block {
                BlockId::Empty => BlockType::Empty,
                id @ BlockId::Basic(_) | id @ BlockId::Dynamic(_) => match registry.get_entity(id, BlockCoord::from(self.position)+ChunkIdx::from_usize(i).into(), commands) {
                    Some(entity) => BlockType::Filled(entity),
                    None => BlockType::Empty,
                },
            }
        }
        result
    }
}

