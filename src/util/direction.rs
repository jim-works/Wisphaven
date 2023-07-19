use bevy::prelude::Vec3;

use crate::world::BlockCoord;

use super::max_component_norm;

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