use std::ops::{Index, IndexMut};

use bevy::prelude::*;

use crate::util::Direction;

use super::BlockType;

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_SIZE_U8: u8 = CHUNK_SIZE as u8;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LODLevel {pub level: usize}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32
}

impl ChunkCoord {
    pub fn new (x: i32, y: i32, z: i32) -> ChunkCoord {
        ChunkCoord { x, y, z }
    }
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new((self.x*CHUNK_SIZE_I32) as f32,(self.y*CHUNK_SIZE_I32) as f32,(self.z*CHUNK_SIZE_I32) as f32)
    }
    pub fn offset(&self, dir: Direction) -> ChunkCoord {
        match dir {
            Direction::PosX => ChunkCoord::new(self.x+1,self.y,self.z),
            Direction::PosY => ChunkCoord::new(self.x,self.y+1,self.z),
            Direction::PosZ => ChunkCoord::new(self.x,self.y,self.z+1),
            Direction::NegX => ChunkCoord::new(self.x-1,self.y,self.z),
            Direction::NegY => ChunkCoord::new(self.x,self.y-1,self.z),
            Direction::NegZ => ChunkCoord::new(self.x,self.y,self.z-1),
        }
    }
    pub fn to_next_lod(&self) -> ChunkCoord {
        ChunkCoord::new(self.x/2,self.y/2,self.z/2)
    }
}

impl From<Vec3> for ChunkCoord {
    fn from(v: Vec3) -> Self {
        ChunkCoord::new((v.x/CHUNK_SIZE_F32).floor() as i32,(v.y/CHUNK_SIZE_F32).floor() as i32,(v.z/CHUNK_SIZE_F32).floor() as i32)
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
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.x as f32,self.y as f32,self.z as f32)
    }
    pub fn to_usize(&self) -> usize {
        (self.x as usize)*CHUNK_SIZE*CHUNK_SIZE+(self.y as usize)*CHUNK_SIZE+(self.z as usize)
    }
}

#[derive(Clone, Debug)]
pub enum ChunkType {
    Ungenerated(Entity),
    Full(Chunk)
}

#[derive(Clone, Debug)]
pub struct Chunk {
    blocks: Box<[BlockType; BLOCKS_PER_CHUNK]>,
    pub position: ChunkCoord,
    pub entity: Entity
}

impl Index<ChunkIdx> for Chunk {
    type Output = BlockType;
    fn index(&self, index: ChunkIdx) -> &BlockType {
        &self.blocks[index.to_usize()]
    }
}

impl IndexMut<ChunkIdx> for Chunk {
    fn index_mut(&mut self, index: ChunkIdx) -> &mut BlockType {
        &mut self.blocks[index.to_usize()]
    }
}

impl Index<usize> for Chunk {
    type Output = BlockType;
    fn index(&self, index: usize) -> &BlockType {
        &self.blocks[index]
    }
}

impl IndexMut<usize> for Chunk {
    fn index_mut(&mut self, index: usize) -> &mut BlockType {
        &mut self.blocks[index]
    }
}

impl Chunk {
    pub fn new(position: ChunkCoord, entity: Entity) -> Chunk {
        Chunk {
            blocks: Box::new([BlockType::Empty; BLOCKS_PER_CHUNK]),
            entity: entity,
            position: position
        }
    }
}
#[derive(Clone, Debug)]
pub enum LODChunkType {
    //entity, level
    Ungenerated(Entity, usize),
    //level, represents a space that is occupied by a lower LOD chunk
    Placeholder(usize),
    Full(LODChunk)
}
#[derive(Clone, Debug)]
pub struct LODChunk {
    blocks: Box<[BlockType; BLOCKS_PER_CHUNK]>,
    pub position: ChunkCoord,
    pub entity: Entity,
    pub level: usize
}

impl Index<ChunkIdx> for LODChunk {
    type Output = BlockType;
    fn index(&self, index: ChunkIdx) -> &BlockType {
        &self.blocks[index.to_usize()]
    }
}

impl IndexMut<ChunkIdx> for LODChunk {
    fn index_mut(&mut self, index: ChunkIdx) -> &mut BlockType {
        &mut self.blocks[index.to_usize()]
    }
}

impl Index<usize> for LODChunk {
    type Output = BlockType;
    fn index(&self, index: usize) -> &BlockType {
        &self.blocks[index]
    }
}

impl IndexMut<usize> for LODChunk {
    fn index_mut(&mut self, index: usize) -> &mut BlockType {
        &mut self.blocks[index]
    }
}

impl LODChunk {
    pub fn new(position: ChunkCoord, level: usize, entity: Entity) -> LODChunk {
        LODChunk {
            blocks: Box::new([BlockType::Empty; BLOCKS_PER_CHUNK]),
            entity,
            position,
            level
        }
    }

    pub fn scale(&self) -> i32 {
        LODChunk::level_to_scale(self.level)
    }

    pub fn level_to_scale(level: usize) -> i32 {
        1<<level
    }

    pub fn get_block_pos(&self, pos: ChunkIdx) -> Vec3 {
        let scale = self.scale();
        Vec3::new((self.position.x*CHUNK_SIZE_I32*scale+(pos.x as i32)*scale) as f32, (self.position.y*CHUNK_SIZE_I32*scale+(pos.y as i32)*scale) as f32, (self.position.z*CHUNK_SIZE_I32*scale+(pos.z as i32)*scale) as f32)
    }
}