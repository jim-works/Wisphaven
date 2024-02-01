use std::{ops::{Index, IndexMut, Add, Div, Sub}, marker::PhantomData};

use bevy::prelude::*;
use serde::{Serialize, Deserialize};

use crate::{util::{palette::BlockPalette, Direction}, BlockMesh};

use super::{BlockType, BlockCoord, BlockId, BlockRegistry, Id};

pub const CHUNK_SIZE: usize = 16;
pub const FAT_CHUNK_SIZE: usize = CHUNK_SIZE+2;
pub const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_SIZE_U8: u8 = CHUNK_SIZE as u8;
pub const CHUNK_SIZE_I8: i8 = CHUNK_SIZE as i8;
pub const CHUNK_SIZE_U64: u64 = CHUNK_SIZE as u64;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE;
//fat chunk contains one layer of information about its neighbors
pub const BLOCKS_PER_FAT_CHUNK: usize = FAT_CHUNK_SIZE*FAT_CHUNK_SIZE*FAT_CHUNK_SIZE;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LODLevel {pub level: u8}

pub type ArrayChunk = Chunk<BlockPalette<BlockType, BLOCKS_PER_CHUNK>, BlockType>;
pub type LODChunk = ArrayChunk;
// pub type GeneratingChunk = Chunk<[BlockId; BLOCKS_PER_CHUNK], BlockId>;
// pub type GeneratingLODChunk = GeneratingChunk;
pub type GeneratingChunk = Chunk<BlockPalette<BlockId, BLOCKS_PER_CHUNK>, BlockId>;
pub type GeneratingLODChunk = GeneratingChunk;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32
}

#[derive(Component, Debug, Copy, Clone)]
pub struct DontMeshChunk;

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

impl Sub<ChunkCoord> for ChunkCoord {
    type Output = ChunkCoord;

    fn sub(self, rhs: ChunkCoord) -> Self::Output {
        ChunkCoord::new(self.x-rhs.x,self.y-rhs.y,self.z-rhs.z)
    }
}


impl Div<i32> for ChunkCoord {
    type Output = ChunkCoord;

    fn div(self, rhs: i32) -> Self::Output {
        ChunkCoord::new(self.x/rhs,self.y/rhs,self.z/rhs)
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
        ChunkIdx { x, y ,z }
    }
    pub fn wrapped(x: u8, y: u8, z: u8) -> ChunkIdx {
        ChunkIdx { x: x%CHUNK_SIZE_U8, y: y%CHUNK_SIZE_U8, z: z%CHUNK_SIZE_U8 }
    }
    pub fn from_usize (i: usize) -> ChunkIdx {
        let x = i/(CHUNK_SIZE*CHUNK_SIZE);
        let y = (i-x*CHUNK_SIZE*CHUNK_SIZE)/CHUNK_SIZE;
        let z = i-x*CHUNK_SIZE*CHUNK_SIZE-y*CHUNK_SIZE;
        ChunkIdx { x: x as u8, y: y as u8, z: z as u8 }
    }
    //will offset by one unit in the given direction, wrapping if overflow
    pub fn offset(self, value: Direction) -> Self {
        match value {
            Direction::PosX => ChunkIdx { x: (self.x+1)%CHUNK_SIZE_U8, y: 0, z: 0 },
            Direction::PosY => ChunkIdx { x: 0, y: (self.y+1)%CHUNK_SIZE_U8, z: 0 },
            Direction::PosZ => ChunkIdx { x: 0, y: 0, z: (self.z+1)%CHUNK_SIZE_U8 },
            Direction::NegX => ChunkIdx { x: self.x.wrapping_sub(1)%CHUNK_SIZE_U8, y: 0, z: 0 },
            Direction::NegY => ChunkIdx { x: 0, y: self.y.wrapping_sub(1)%CHUNK_SIZE_U8, z: 0 },
            Direction::NegZ => ChunkIdx { x: 0, y: 0, z: self.z.wrapping_sub(1)%CHUNK_SIZE_U8 },
        }
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

impl From<ChunkIdx> for usize {
    fn from(value: ChunkIdx) -> Self {
        (value.x as usize)*CHUNK_SIZE*CHUNK_SIZE+(value.y as usize)*CHUNK_SIZE+(value.z as usize)
    }
}

impl Add<ChunkIdx> for ChunkIdx {
    type Output = Self;

    fn add(self, rhs: ChunkIdx) -> Self::Output {
        ChunkIdx::new(self.x+rhs.x,self.y+rhs.y,self.z+rhs.z)
    }
}

//index for chunk with extra layer (CHUNK_SIZE+2)*(CHUNK_SIZE+2)*(CHUNK_SIZE+2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FatChunkIdx {
    pub x: i8,
    pub y: i8,
    pub z: i8
}

impl FatChunkIdx {
    pub fn new (x: i8, y: i8, z: i8) -> FatChunkIdx {
        FatChunkIdx { x, y ,z }
    }
}

impl From<FatChunkIdx> for usize {
    fn from(value: FatChunkIdx) -> usize {
        (value.x+1) as usize*FAT_CHUNK_SIZE*FAT_CHUNK_SIZE+(value.y+1) as usize*FAT_CHUNK_SIZE+(value.z+1) as usize
    }
}

impl From<ChunkIdx> for FatChunkIdx {
    fn from(value: ChunkIdx) -> FatChunkIdx {
        FatChunkIdx { x: value.x as i8, y: value.y as i8, z: value.z as i8 }
    }
}

impl From<FatChunkIdx> for ChunkIdx {
    fn from(value: FatChunkIdx) -> ChunkIdx {
        assert!(value.x >= 0);
        assert!(value.x < CHUNK_SIZE_I8);
        assert!(value.y >= 0);
        assert!(value.y < CHUNK_SIZE_I8);
        assert!(value.z >= 0);
        assert!(value.z < CHUNK_SIZE_I8);
        ChunkIdx { x: value.x as u8, y: value.y as u8, z: value.z as u8 }
    }
}

#[derive(Clone, Debug)]
pub enum ChunkType {
    Ungenerated(Entity),
    Generating(crate::worldgen::GenerationPhase, GeneratingChunk),
    Full(ArrayChunk)
}

#[derive(Clone, Debug)]
pub enum LODChunkType {
    //entity, level
    Ungenerated(Entity, u8),
    Full(ArrayChunk)
}

pub trait ChunkStorage<Block>: Index<usize, Output=Block> {
    fn set_block(&mut self, index: usize, val: Block);
}
impl<T, Block> ChunkStorage<Block> for T where T: Index<usize, Output=Block> + IndexMut<usize, Output=Block> {
    fn set_block(&mut self, index: usize, val: Block) {
        self[index] = val;
    }
}
impl<Storage,Block,Idx> IndexMut<Idx> for Chunk<Storage,Block> where Storage: ChunkStorage<Block> + IndexMut<Idx, Output=Block>, Block: ChunkBlock, Chunk<Storage,Block>: Index<Idx, Output=Block> {
    fn index_mut(&mut self, index: Idx) -> &mut Block {
        &mut self.blocks[index]
    }
}

pub trait ChunkBlock: Clone + Send + Sync + PartialEq {}
impl<T> ChunkBlock for T where T: Clone + Send + Sync + PartialEq {}

pub trait ChunkTrait<Block: PartialEq>: Index<ChunkIdx> + Index<usize> {
    fn scale(&self) -> i32;
    fn get_block_pos(&self, pos: ChunkIdx) -> Vec3;
    fn set_block(&mut self, idx: usize, block: Block);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk<Storage, Block> where Storage: ChunkStorage<Block>, Block: ChunkBlock {
    pub blocks: Box<Storage>,
    pub position: ChunkCoord,
    pub entity: Entity,
    //lod level, scale of chunk is 2^level
    pub level: u8,
    //not sure how to get around this
    pub _data: PhantomData<Block>
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> Chunk<Storage, Block> {
    pub fn with_storage<NewBlock: ChunkBlock, NewStorage: ChunkStorage<NewBlock>>(&self, storage: Box<NewStorage>) -> Chunk<NewStorage,NewBlock> {
        Chunk {
            blocks: storage,
            position: self.position,
            entity: self.entity,
            level: self.level,
            _data: PhantomData
        }
    }
    //writes all the data in `with` into `self` except for the entity
    pub fn overwrite(&mut self, with: Self) {
        self.blocks = with.blocks;
        self.position = with.position;
        self.level = with.level;
    }

    //copies entity from `from` into `self`
    pub fn take_metadata<FromStorage: ChunkStorage<FromBlock>, FromBlock: ChunkBlock>(&mut self, from: &Chunk<FromStorage,FromBlock>) {
        self.entity = from.entity;
    }
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> ChunkTrait<Block> for Chunk<Storage, Block> {
    fn scale(&self) -> i32 {
        LODChunk::level_to_scale(self.level)
    }

    fn get_block_pos(&self, pos: ChunkIdx) -> Vec3 {
        let scale = self.scale();
        Vec3::new((self.position.x*CHUNK_SIZE_I32*scale+(pos.x as i32)*scale) as f32, (self.position.y*CHUNK_SIZE_I32*scale+(pos.y as i32)*scale) as f32, (self.position.z*CHUNK_SIZE_I32*scale+(pos.z as i32)*scale) as f32)
    }

    fn set_block(&mut self, idx: usize, block: Block) {
        self.blocks.set_block(idx, block);
    }
    
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> Index<ChunkIdx> for Chunk<Storage, Block> {
    type Output = Block;
    fn index(&self, index: ChunkIdx) -> &Block {
        &self.blocks[index.to_usize()]
    }
}

impl<Storage: ChunkStorage<Block>, Block: ChunkBlock> Index<usize> for Chunk<Storage, Block> {
    type Output = Block;
    fn index(&self, index: usize) -> &Block {
        &self.blocks[index]
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
impl ArrayChunk {
    pub fn new(position: ChunkCoord, entity: Entity) -> ArrayChunk {
        Chunk {
            blocks: Box::new(BlockPalette::new(BlockType::Empty)),
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
            blocks: Box::new(BlockPalette::new(BlockId::default())),
            entity,
            position,
            level: 1,
            _data: PhantomData
        }
    }

    pub fn to_array_chunk(&self, registry: &BlockRegistry, commands: &mut Commands) -> ArrayChunk {
        let mut mapped_palette = Vec::with_capacity(self.blocks.palette.len());
        for (key,val,r) in self.blocks.palette.iter() {
            let block = match val {
                BlockId(Id::Empty) => BlockType::Empty,
                id @ BlockId(Id::Basic(_)) | id @ BlockId(Id::Dynamic(_)) => match registry.get_entity(*id, commands) {
                    Some(entity) => BlockType::Filled(entity),
                    None => BlockType::Empty,
                },
            };
            mapped_palette.push((*key,block,*r));
        }
        self.with_storage(Box::new(BlockPalette { data: self.blocks.data, palette: mapped_palette }))
    }
}
