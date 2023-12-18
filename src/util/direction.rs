use bevy::prelude::{Vec3, IVec3};
use bitflags::bitflags;

use crate::world::{BlockCoord, chunk::{ChunkCoord, ChunkIdx, CHUNK_SIZE_U8, FatChunkIdx, CHUNK_SIZE_I8}};

use super::{max_component_norm, min_component_norm};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Direction{
    PosX,
    PosY,
    PosZ,
    NegX,
    NegY,
    NegZ
}

#[derive(Clone, Copy)]
pub struct DirectionIterator {
    curr: Option<Direction>
}

impl Iterator for DirectionIterator {
    type Item = Direction;
    fn next(&mut self) -> Option<Self::Item> {
        self.curr = match self.curr {
            None => Some(Direction::PosX),
            Some(Direction::PosX) => Some(Direction::PosY),
            Some(Direction::PosY) => Some(Direction::PosZ),
            Some(Direction::PosZ) => Some(Direction::NegX),
            Some(Direction::NegX) => Some(Direction::NegY),
            Some(Direction::NegY) => Some(Direction::NegZ),
            Some(Direction::NegZ) => None,
        };
        self.curr
    }
}

impl Direction {
    pub fn to_idx(self) -> usize {
        match self {
            Direction::PosX => 0,
            Direction::PosY => 1,
            Direction::PosZ => 2,
            Direction::NegX => 3,
            Direction::NegY => 4,
            Direction::NegZ => 5,
        }
    }

    pub fn to_vec3(self) -> Vec3 {
        match self {
            Direction::PosX => Vec3::new(1.0,0.0,0.0),
            Direction::PosY => Vec3::new(0.0,1.0,0.0),
            Direction::PosZ => Vec3::new(0.0,0.0,1.0),
            Direction::NegX => Vec3::new(-1.0,0.0,0.0),
            Direction::NegY => Vec3::new(0.0,-1.0,0.0),
            Direction::NegZ => Vec3::new(0.0,0.0,-1.0),
        }
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::PosX => Direction::NegX,
            Direction::PosY => Direction::NegY,
            Direction::PosZ => Direction::NegZ,
            Direction::NegX => Direction::PosX,
            Direction::NegY => Direction::PosY,
            Direction::NegZ => Direction::PosZ,
        }
    }

    //calls f for each lattice point in the (2*radius x 2*radius) grid perpendicular to this direction
    //result will always be 0 in self's axis
    pub fn for_each_in_plane(self, radius: i32, mut f: impl FnMut(BlockCoord)) {
        match self {
            Direction::PosX | Direction::NegX => for z in -radius..radius+1 {
                for y in -radius..radius+1 {
                    (f)(BlockCoord::new(0,y,z));
                }
            },
            Direction::PosY | Direction::NegY => for x in -radius..radius+1 {
                for z in -radius..radius+1 {
                    (f)(BlockCoord::new(x,0,z));
                }
            },
            Direction::PosZ | Direction::NegZ => for x in -radius..radius+1 {
                for y in -radius..radius+1 {
                    (f)(BlockCoord::new(x,y,0));
                }
            },
        }
    }

    pub fn get_axis(self, v: Vec3) -> f32 {
        match self {
            Direction::PosX | Direction::NegX => v.x,
            Direction::PosY | Direction::NegY => v.y,
            Direction::PosZ | Direction::NegZ => v.z,
        }
    }

    pub fn min_magnitude_axis(v: Vec3) -> Self {
        let min = min_component_norm(v);
        if min.x > 0.0 {
            Direction::PosX
        } else if min.x < 0.0 {
            Direction::NegX
        } else if min.y > 0.0 {
            Direction::PosY
        } else if min.y < 0.0 {
            Direction::NegY
        } else if min.z > 0.0 {
            Direction::PosZ
        } else {
            Direction::NegZ
        }
    }

    pub fn iter() -> DirectionIterator {
        DirectionIterator { curr: None }
    }
}

impl From<u64> for Direction {
    fn from(value: u64) -> Self {
        match value % 6 {
            0 => Direction::PosX,
            1 => Direction::PosY,
            2 => Direction::PosZ,
            3 => Direction::NegX,
            4 => Direction::NegY,
            5 => Direction::NegZ,
            //shouldn't happen
            _ => unreachable!()
        }
    }
}

impl From<usize> for Direction {
    fn from(value: usize) -> Self {
        match value % 6 {
            0 => Direction::PosX,
            1 => Direction::PosY,
            2 => Direction::PosZ,
            3 => Direction::NegX,
            4 => Direction::NegY,
            5 => Direction::NegZ,
            //shouldn't happen
            _ => unreachable!()
        }
    }
}

impl From<Vec3> for Direction {
    fn from(value: Vec3) -> Self {
        let max = max_component_norm(value);
        if max.x > 0.0 {
            Direction::PosX
        } else if max.x < 0.0 {
            Direction::NegX
        } else if max.y > 0.0 {
            Direction::PosY
        } else if max.y < 0.0 {
            Direction::NegY
        } else if max.z > 0.0 {
            Direction::PosZ
        } else {
            Direction::NegZ
        }
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy)]
    pub struct DirectionFlags : u8 {
        const PosX = 0b000001;
        const PosY = 0b000010;
        const PosZ = 0b000100;
        const NegX = 0b001000;
        const NegY = 0b010000;
        const NegZ = 0b100000;
    }
}

impl From<Direction> for DirectionFlags {
    fn from(value: Direction) -> Self {
        match value {
            Direction::PosX => DirectionFlags::PosX,
            Direction::PosY => DirectionFlags::PosY,
            Direction::PosZ => DirectionFlags::PosZ,
            Direction::NegX => DirectionFlags::NegX,
            Direction::NegY => DirectionFlags::NegY,
            Direction::NegZ => DirectionFlags::NegZ,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Corner {
    NXNYNZ=0,
    NXNYPZ=1,
    NXPYNZ=2,
    NXPYPZ=3,
    PXNYNZ=4,
    PXNYPZ=5,
    PXPYNZ=6,
    PXPYPZ=7,
}
#[derive(Clone, Copy)]
pub struct CornerIterator {
    curr: Option<Corner>
}

impl Iterator for CornerIterator {
    type Item = Corner;
    fn next(&mut self) -> Option<Self::Item> {
        self.curr = match self.curr {
            None => Some(Corner::NXNYNZ),
            Some(Corner::NXNYNZ) => Some(Corner::NXNYPZ),
            Some(Corner::NXNYPZ) => Some(Corner::NXPYNZ),
            Some(Corner::NXPYNZ) => Some(Corner::NXPYPZ),
            Some(Corner::NXPYPZ) => Some(Corner::PXNYNZ),
            Some(Corner::PXNYNZ) => Some(Corner::PXNYPZ),
            Some(Corner::PXNYPZ) => Some(Corner::PXPYNZ),
            Some(Corner::PXPYNZ) => Some(Corner::PXPYPZ),
            Some(Corner::PXPYPZ) => None,
        };
        self.curr
    }
}

impl Corner {
    pub fn iter() -> CornerIterator {
        CornerIterator { curr: None }
    }
    pub fn opposite(self) -> Corner {
        match self {
            Corner::NXNYNZ => Corner::PXPYPZ,
            Corner::NXNYPZ => Corner::PXPYNZ,
            Corner::NXPYNZ => Corner::PXNYPZ,
            Corner::NXPYPZ => Corner::PXNYNZ,
            Corner::PXNYNZ => Corner::NXPYPZ,
            Corner::PXNYPZ => Corner::NXPYNZ,
            Corner::PXPYNZ => Corner::NXNYPZ,
            Corner::PXPYPZ => Corner::NXNYNZ,
        }
    }
}

impl From<Corner> for ChunkCoord {
    fn from(value: Corner) -> Self {
        match value {
            Corner::NXNYNZ => ChunkCoord::new(-1,-1,-1),
            Corner::NXNYPZ => ChunkCoord::new(-1,-1,1),
            Corner::NXPYNZ => ChunkCoord::new(-1,1,-1),
            Corner::NXPYPZ => ChunkCoord::new(-1,1,1),
            Corner::PXNYNZ => ChunkCoord::new(1,-1,-1),
            Corner::PXNYPZ => ChunkCoord::new(1,-1,1),
            Corner::PXPYNZ => ChunkCoord::new(1,1,-1),
            Corner::PXPYPZ => ChunkCoord::new(1,1,1),
        }
    }
}

impl From<Corner> for ChunkIdx {
    fn from(value: Corner) -> Self {
        match value {
            Corner::NXNYNZ => ChunkIdx::new(0,0,0),
            Corner::NXNYPZ => ChunkIdx::new(0,0,CHUNK_SIZE_U8-1),
            Corner::NXPYNZ => ChunkIdx::new(0,CHUNK_SIZE_U8-1,0),
            Corner::NXPYPZ => ChunkIdx::new(0,CHUNK_SIZE_U8-1,CHUNK_SIZE_U8-1),
            Corner::PXNYNZ => ChunkIdx::new(CHUNK_SIZE_U8-1,0,0),
            Corner::PXNYPZ => ChunkIdx::new(CHUNK_SIZE_U8-1,0,CHUNK_SIZE_U8-1),
            Corner::PXPYNZ => ChunkIdx::new(CHUNK_SIZE_U8-1,CHUNK_SIZE_U8-1,0),
            Corner::PXPYPZ => ChunkIdx::new(CHUNK_SIZE_U8-1,CHUNK_SIZE_U8-1,CHUNK_SIZE_U8-1),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Edge {
    NXFaceNY=0,
    NXFacePZ=1,
    NXFacePY=2,
    NXFaceNZ=3,
    PXFaceNY=4,
    PXFacePZ=5,
    PXFacePY=6,
    PXFaceNZ=7,
    NYFaceNZ=8,
    NYFacePZ=9,
    PYFaceNZ=10,
    PYFacePZ=11
}
#[derive(Clone, Copy)]
pub struct EdgeIterator {
    curr: Option<Edge>
}

impl Iterator for EdgeIterator {
    type Item = Edge;
    fn next(&mut self) -> Option<Self::Item> {
        self.curr = match self.curr {
            None => Some(Edge::NXFaceNY),
            Some(Edge::NXFaceNY) => Some(Edge::NXFacePZ),
            Some(Edge::NXFacePZ) => Some(Edge::NXFacePY),
            Some(Edge::NXFacePY) => Some(Edge::NXFaceNZ),
            Some(Edge::NXFaceNZ) => Some(Edge::PXFaceNY),
            Some(Edge::PXFaceNY) => Some(Edge::PXFacePZ),
            Some(Edge::PXFacePZ) => Some(Edge::PXFacePY),
            Some(Edge::PXFacePY) => Some(Edge::PXFaceNZ),
            Some(Edge::PXFaceNZ) => Some(Edge::NYFaceNZ),
            Some(Edge::NYFaceNZ) => Some(Edge::NYFacePZ),
            Some(Edge::NYFacePZ) => Some(Edge::PYFaceNZ),
            Some(Edge::PYFaceNZ) => Some(Edge::PYFacePZ),
            Some(Edge::PYFacePZ) => None,
        };
        self.curr
    }
}

impl Edge {
    pub fn iter() -> EdgeIterator {
        EdgeIterator { curr: None }
    }
    pub fn opposite(self) -> Edge {
        match self {
            Edge::NXFaceNY => Edge::PXFacePY,
            Edge::NXFacePZ => Edge::PXFaceNZ,
            Edge::NXFacePY => Edge::PXFaceNY,
            Edge::NXFaceNZ => Edge::PXFacePZ,
            Edge::PXFaceNY => Edge::NXFacePY,
            Edge::PXFacePZ => Edge::NXFaceNZ,
            Edge::PXFacePY => Edge::NXFaceNY,
            Edge::PXFaceNZ => Edge::NXFacePZ,
            Edge::NYFaceNZ => Edge::PYFacePZ,
            Edge::NYFacePZ => Edge::PYFaceNZ,
            Edge::PYFaceNZ => Edge::NYFacePZ,
            Edge::PYFacePZ => Edge::NYFaceNZ,
        }
    }
    pub fn direction(self) -> IVec3 {
        match self {
            Edge::NXFaceNY => IVec3::new(0,0,1),
            Edge::NXFacePZ => IVec3::new(0,1,0),
            Edge::NXFacePY => IVec3::new(0,0,1),
            Edge::NXFaceNZ => IVec3::new(0,1,0),
            Edge::PXFaceNY => IVec3::new(0,0,1),
            Edge::PXFacePZ => IVec3::new(0,1,0),
            Edge::PXFacePY => IVec3::new(0,0,1),
            Edge::PXFaceNZ => IVec3::new(0,1,0),
            Edge::NYFaceNZ => IVec3::new(1,0,0),
            Edge::NYFacePZ => IVec3::new(1,0,0),
            Edge::PYFaceNZ => IVec3::new(1,0,0),
            Edge::PYFacePZ => IVec3::new(1,0,0),
        }
    }
    //edges for a normal sized chunk
    pub fn origin(self) -> ChunkIdx {
        match self {
            Edge::NXFaceNY => ChunkIdx::new(0,0,0),
            Edge::NXFacePZ => ChunkIdx::new(0,0,CHUNK_SIZE_U8-1),
            Edge::NXFacePY => ChunkIdx::new(0,CHUNK_SIZE_U8-1,0),
            Edge::NXFaceNZ => ChunkIdx::new(0,0,0),
            Edge::PXFaceNY => ChunkIdx::new(CHUNK_SIZE_U8-1,0,0),
            Edge::PXFacePZ => ChunkIdx::new(CHUNK_SIZE_U8-1,0,CHUNK_SIZE_U8-1),
            Edge::PXFacePY => ChunkIdx::new(CHUNK_SIZE_U8-1,CHUNK_SIZE_U8-1,0),
            Edge::PXFaceNZ => ChunkIdx::new(CHUNK_SIZE_U8-1,0,0),
            Edge::NYFaceNZ => ChunkIdx::new(0,0,0),
            Edge::NYFacePZ => ChunkIdx::new(0,0,CHUNK_SIZE_U8-1),
            Edge::PYFaceNZ => ChunkIdx::new(0,CHUNK_SIZE_U8-1,0),
            Edge::PYFacePZ => ChunkIdx::new(0,CHUNK_SIZE_U8-1,CHUNK_SIZE_U8-1),
        }
    }
    //edges for a fat chunk
    pub fn fat_origin(self) -> FatChunkIdx {
        match self {
            Edge::NXFaceNY => FatChunkIdx::new(-1,-1,-1),
            Edge::NXFacePZ => FatChunkIdx::new(-1,-1,CHUNK_SIZE_I8),
            Edge::NXFacePY => FatChunkIdx::new(-1,CHUNK_SIZE_I8,-1),
            Edge::NXFaceNZ => FatChunkIdx::new(-1,-1,-1),
            Edge::PXFaceNY => FatChunkIdx::new(CHUNK_SIZE_I8,-1,-1),
            Edge::PXFacePZ => FatChunkIdx::new(CHUNK_SIZE_I8,-1,CHUNK_SIZE_I8),
            Edge::PXFacePY => FatChunkIdx::new(CHUNK_SIZE_I8,CHUNK_SIZE_I8,-1),
            Edge::PXFaceNZ => FatChunkIdx::new(CHUNK_SIZE_I8,-1,-1),
            Edge::NYFaceNZ => FatChunkIdx::new(-1,-1,-1),
            Edge::NYFacePZ => FatChunkIdx::new(-1,-1,CHUNK_SIZE_I8),
            Edge::PYFaceNZ => FatChunkIdx::new(-1,CHUNK_SIZE_I8,-1),
            Edge::PYFacePZ => FatChunkIdx::new(-1,CHUNK_SIZE_I8,CHUNK_SIZE_I8),
        }
    }
}

impl From<Edge> for ChunkCoord {
    fn from(value: Edge) -> Self {
        match value {
            Edge::NXFaceNY => ChunkCoord::new(-1,-1,0),
            Edge::NXFacePZ => ChunkCoord::new(-1,0,1),
            Edge::NXFacePY => ChunkCoord::new(-1,1,0),
            Edge::NXFaceNZ => ChunkCoord::new(-1,0,-1),
            Edge::PXFaceNY => ChunkCoord::new(1,-1,0),
            Edge::PXFacePZ => ChunkCoord::new(1,0,1),
            Edge::PXFacePY => ChunkCoord::new(1,1,0),
            Edge::PXFaceNZ => ChunkCoord::new(1,0,-1),
            Edge::NYFaceNZ => ChunkCoord::new(0,-1,-1),
            Edge::NYFacePZ => ChunkCoord::new(0,-1,1),
            Edge::PYFaceNZ => ChunkCoord::new(0,1,-1),
            Edge::PYFacePZ => ChunkCoord::new(0,1,1),
        }
    }
}