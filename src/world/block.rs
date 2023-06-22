use std::ops::AddAssign;

use crate::util::Direction;
use bevy::prelude::*;

use super::chunk::{ChunkCoord, CHUNK_SIZE_I32};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
    #[default]
    Empty,
    Basic(u32),
    Entity(Entity),
}

//marker for components to look for on a BlockEntity
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlockData {
    Storage,
}

#[derive(Component)]
pub struct BlockEntity(Vec<BlockData>);

pub enum BlockMesh {
    //standard block shape with all sides being the same texture
    Uniform(u32),
    //standard block shape with each side having a unique texture
    MultiTexture([u32; 6]),
    //Slab with height from bottom (1.0) is the same as uniform, (0.0) is empty
    BottomSlab(f32, [u32; 6]),
}

#[derive(Resource)]
pub struct BlockRegistry {
    pub meshes: Vec<BlockMesh>,
}

impl BlockRegistry {
    pub fn get_block_mesh(&self, id: u32) -> &BlockMesh {
        &self.meshes[id as usize]
    }
    pub fn is_transparent(&self, block: BlockType, face: Direction) -> bool {
        match block {
            BlockType::Empty => true,
            BlockType::Basic(id) => match self.meshes[id as usize] {
                BlockMesh::Uniform(_) => false,
                BlockMesh::MultiTexture(_) => false,
                BlockMesh::BottomSlab(_, _) => face != Direction::NegY,
            },
            BlockType::Entity(_) => todo!(),
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockCoord {
    pub fn new(x: i32, y: i32, z: i32) -> BlockCoord {
        BlockCoord { x, y, z }
    }
    //returns coordinate at negative corner of block
    pub fn to_vec3(self) -> Vec3 {
        Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }
    //returns coordinate at center of block
    pub fn center(self) -> Vec3 {
        Vec3::new(
            self.x as f32 + 0.5,
            self.y as f32 + 0.5,
            self.z as f32 + 0.5,
        )
    }
    pub fn offset(self, dir: Direction) -> BlockCoord {
        match dir {
            Direction::PosX => BlockCoord::new(self.x + 1, self.y, self.z),
            Direction::PosY => BlockCoord::new(self.x, self.y + 1, self.z),
            Direction::PosZ => BlockCoord::new(self.x, self.y, self.z + 1),
            Direction::NegX => BlockCoord::new(self.x - 1, self.y, self.z),
            Direction::NegY => BlockCoord::new(self.x, self.y - 1, self.z),
            Direction::NegZ => BlockCoord::new(self.x, self.y, self.z - 1),
        }
    }
    //if v has maximum element m, returns the vector with m set to sign(m) and all other elements 0.
    pub fn max_component_norm(self) -> BlockCoord {
        let abs = self.abs();
        if abs.x > abs.y && abs.x > abs.z {
            BlockCoord::new(self.x.signum(), 0, 0)
        } else if abs.y > abs.z {
            BlockCoord::new(0, self.y.signum(), 0)
        } else {
            BlockCoord::new(0, 0, self.z.signum())
        }
    }
    pub fn abs(self) -> BlockCoord {
        BlockCoord::new(self.x.abs(), self.y.abs(), self.z.abs())
    }
}

impl std::ops::Add<BlockCoord> for BlockCoord {
    type Output = Self;
    fn add(self, rhs: BlockCoord) -> Self::Output {
        BlockCoord::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub<BlockCoord> for BlockCoord {
    type Output = Self;
    fn sub(self, rhs: BlockCoord) -> Self::Output {
        BlockCoord::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl AddAssign for BlockCoord {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl From<Vec3> for BlockCoord {
    fn from(v: Vec3) -> Self {
        BlockCoord::new(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32)
    }
}

impl From<ChunkCoord> for BlockCoord {
    fn from(v: ChunkCoord) -> Self {
        BlockCoord::new(
            v.x * CHUNK_SIZE_I32,
            v.y * CHUNK_SIZE_I32,
            v.z * CHUNK_SIZE_I32,
        )
    }
}
