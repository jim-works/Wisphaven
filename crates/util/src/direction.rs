use bevy::{
    math::Dir3,
    prelude::{IVec3, Vec3},
};
use bitflags::bitflags;

use crate::max_component;

use super::{max_component_norm, min_component_norm};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Direction {
    PosX,
    PosY,
    PosZ,
    NegX,
    NegY,
    NegZ,
}

#[derive(Clone, Copy)]
pub struct DirectionIterator {
    curr: Option<Direction>,
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
            Direction::PosX => Vec3::new(1.0, 0.0, 0.0),
            Direction::PosY => Vec3::new(0.0, 1.0, 0.0),
            Direction::PosZ => Vec3::new(0.0, 0.0, 1.0),
            Direction::NegX => Vec3::new(-1.0, 0.0, 0.0),
            Direction::NegY => Vec3::new(0.0, -1.0, 0.0),
            Direction::NegZ => Vec3::new(0.0, 0.0, -1.0),
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
    pub fn for_each_in_plane(self, radius: i32, mut f: impl FnMut(IVec3)) {
        match self {
            Direction::PosX | Direction::NegX => {
                for z in -radius..radius + 1 {
                    for y in -radius..radius + 1 {
                        (f)(IVec3::new(0, y, z));
                    }
                }
            }
            Direction::PosY | Direction::NegY => {
                for x in -radius..radius + 1 {
                    for z in -radius..radius + 1 {
                        (f)(IVec3::new(x, 0, z));
                    }
                }
            }
            Direction::PosZ | Direction::NegZ => {
                for x in -radius..radius + 1 {
                    for y in -radius..radius + 1 {
                        (f)(IVec3::new(x, y, 0));
                    }
                }
            }
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
            _ => unreachable!(),
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
            _ => unreachable!(),
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

impl From<Dir3> for Direction {
    fn from(value: Dir3) -> Self {
        let max = max_component(value);
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
    NXNYNZ = 0,
    NXNYPZ = 1,
    NXPYNZ = 2,
    NXPYPZ = 3,
    PXNYNZ = 4,
    PXNYPZ = 5,
    PXPYNZ = 6,
    PXPYPZ = 7,
}
#[derive(Clone, Copy)]
pub struct CornerIterator {
    curr: Option<Corner>,
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

impl From<Corner> for IVec3 {
    fn from(value: Corner) -> Self {
        match value {
            Corner::NXNYNZ => IVec3::new(-1, -1, -1),
            Corner::NXNYPZ => IVec3::new(-1, -1, 1),
            Corner::NXPYNZ => IVec3::new(-1, 1, -1),
            Corner::NXPYPZ => IVec3::new(-1, 1, 1),
            Corner::PXNYNZ => IVec3::new(1, -1, -1),
            Corner::PXNYPZ => IVec3::new(1, -1, 1),
            Corner::PXPYNZ => IVec3::new(1, 1, -1),
            Corner::PXPYPZ => IVec3::new(1, 1, 1),
        }
    }
}
