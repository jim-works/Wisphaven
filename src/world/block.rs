use bevy::prelude::*;
use crate::util::Direction;

use super::chunk::{ChunkCoord, CHUNK_SIZE_I32};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
    #[default]
    Empty,
    Basic(u32)
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32
}

impl BlockCoord {
    pub fn new (x: i32, y: i32, z: i32) -> BlockCoord {
        BlockCoord { x, y, z }
    }
    //returns coordinate at negative corner of block
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.x as f32,self.y as f32,self.z as f32)
    }
    //returns coordinate at center of block
    pub fn center(&self) -> Vec3 {
        Vec3::new(self.x as f32+0.5,self.y as f32+0.5,self.z as f32+0.5)
    }
    pub fn offset(&self, dir: Direction) -> BlockCoord {
        match dir {
            Direction::PosX => BlockCoord::new(self.x+1,self.y,self.z),
            Direction::PosY => BlockCoord::new(self.x,self.y+1,self.z),
            Direction::PosZ => BlockCoord::new(self.x,self.y,self.z+1),
            Direction::NegX => BlockCoord::new(self.x-1,self.y,self.z),
            Direction::NegY => BlockCoord::new(self.x,self.y-1,self.z),
            Direction::NegZ => BlockCoord::new(self.x,self.y,self.z-1),
        }
    }
}

impl std::ops::Add<BlockCoord> for BlockCoord {
    type Output = Self;
    fn add(self, rhs: BlockCoord) -> Self::Output {
        BlockCoord::new(self.x+rhs.x, self.y+rhs.y, self.z+rhs.z)
    }
}

impl From<Vec3> for BlockCoord {
    fn from(v: Vec3) -> Self {
        BlockCoord::new(v.x.floor() as i32,v.y.floor() as i32,v.z.floor() as i32)
    }
}

impl From<ChunkCoord> for BlockCoord {
    fn from(v: ChunkCoord) -> Self {
        BlockCoord::new(v.x*CHUNK_SIZE_I32,v.y*CHUNK_SIZE_I32,v.z*CHUNK_SIZE_I32)
    }
}