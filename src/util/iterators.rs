use crate::world::{BlockCoord, BlockType};
use bevy::prelude::*;

pub struct BlockVolumeIterator {
    x_len: i32,
    y_len: i32,
    z_len: i32,
    x_i: i32,
    y_i: i32,
    z_i: i32,
    done: bool, //this is ugly but i'm tired
}

impl BlockVolumeIterator {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            x_len: x as i32,
            y_len: y as i32,
            z_len: z as i32,
            x_i: 0,
            y_i: 0,
            z_i: 0,
            done: x == 0 || y == 0 || z == 0,
        }
    }

    pub fn from_volume(volume: BlockVolume) -> impl Iterator<Item=BlockCoord> {
        let size = volume.max_corner - volume.min_corner;
        Self {
            x_len: size.x,
            y_len: size.y,
            z_len: size.z,
            x_i: 0,
            y_i: 0,
            z_i: 0,
            done: size.x <= 0 || size.y <= 0 || size.z <= 0,
        }
        .map(move |offset| volume.min_corner + offset)
    }
}

impl Iterator for BlockVolumeIterator {
    type Item = BlockCoord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let ret = Some(BlockCoord::new(self.x_i, self.y_i, self.z_i));
        self.x_i += 1;
        if self.x_i >= self.x_len {
            self.y_i += 1;
            self.x_i = 0;
        }
        if self.y_i >= self.y_len {
            self.z_i += 1;
            self.y_i = 0;
        }
        if self.z_i >= self.z_len {
            self.done = true;
        }
        ret
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockVolume {
    pub min_corner: BlockCoord,
    pub max_corner: BlockCoord,
}

impl BlockVolume {
    //returns true if min <= other min and max >= other max.
    //contains itself!
    pub fn contains(&self, other: BlockVolume) -> bool {
        (self.min_corner.x <= other.min_corner.x
            && self.min_corner.y <= other.min_corner.y
            && self.min_corner.z <= other.min_corner.z)
            && (self.max_corner.x >= other.max_corner.x
                && self.max_corner.y >= other.max_corner.y
                && self.max_corner.z >= other.max_corner.z)
    }

    pub fn intersects(&self, other: BlockVolume) -> bool {
        (self.min_corner.x <= other.max_corner.x && self.max_corner.x >= other.min_corner.x)
            && (self.min_corner.y <= other.max_corner.y && self.max_corner.y >= other.min_corner.y)
            && (self.min_corner.z <= other.max_corner.z && self.max_corner.z >= other.min_corner.z)
    }

    pub fn volume(&self) -> i32 {
        (self.max_corner.x - self.min_corner.x)
            * (self.max_corner.y - self.min_corner.y)
            * (self.max_corner.z - self.min_corner.z)
    }

    pub fn size(&self) -> BlockCoord {
        self.max_corner - self.min_corner
    }

    pub fn center(&self) -> Vec3 {
        self.min_corner.to_vec3() + self.size().to_vec3()/2.0
    }

    pub fn new(min_corner: BlockCoord, max_corner_exclusive: BlockCoord) -> Self {
        BlockVolume {
            min_corner,
            max_corner: max_corner_exclusive,
        }
    }

    pub fn new_inclusive(min_corner: BlockCoord, max_corner_inclusive: BlockCoord) -> Self {
        BlockVolume {
            min_corner,
            max_corner: max_corner_inclusive+BlockCoord::new(1,1,1),
        }
    }

    pub fn iter(self) -> impl Iterator<Item = BlockCoord> {
        BlockVolumeIterator::from_volume(self)
    }
}

pub struct BlockVolumeContainer {
    blocks: Vec<Option<BlockType>>,
    volume: BlockVolume,
    size: BlockCoord,
}

impl BlockVolumeContainer {
    pub fn new(volume: BlockVolume) -> Self {
        Self {
            blocks: vec![None; volume.volume() as usize],
            volume,
            size: volume.max_corner - volume.min_corner,
        }
    }

    pub fn volume(&self) -> BlockVolume {
        self.volume
    }

    pub fn size(&self) -> BlockCoord {
        self.size
    }

    pub fn iter(&self) -> impl Iterator<Item = (BlockCoord, Option<BlockType>)> + '_ {
        self.volume.iter().map(|pos| (pos, self[pos]))
    }

    //clears blocks, and reuses buffer for new volume, expanding if needed
    pub fn recycle(&mut self, volume: BlockVolume) {
        self.volume = volume;
        self.size = volume.max_corner - volume.min_corner;
        self.blocks.clear();
        self.blocks.resize(volume.volume() as usize, None);
    }
}

impl std::ops::Index<BlockCoord> for BlockVolumeContainer {
    type Output = Option<BlockType>;

    fn index(&self, mut index: BlockCoord) -> &Self::Output {
        index -= self.volume.min_corner;
        &self.blocks
            [(index.x + index.y * self.size.x + index.z * self.size.x * self.size.y) as usize]
    }
}

impl std::ops::IndexMut<BlockCoord> for BlockVolumeContainer {
    fn index_mut(&mut self, mut index: BlockCoord) -> &mut Self::Output {
        index -= self.volume.min_corner;
        &mut self.blocks
            [(index.x + index.y * self.size.x + index.z * self.size.x * self.size.y) as usize]
    }
}

#[test]
fn test_volume_iterator() {
    let it = BlockVolumeIterator::new(5, 6, 7);
    let mut count = 0;
    for coord in it {
        count += 1;
        assert!(coord.x < 5);
        assert!(coord.y < 6);
        assert!(coord.z < 7);
    }
    assert_eq!(count, 5 * 6 * 7);
}

#[test]
fn test_volume_iterator_zero() {
    let it = BlockVolumeIterator::new(0, 0, 0);
    let mut count = 0;
    for _ in it {
        count += 1;
    }
    assert_eq!(count, 0);
}

#[test]
fn test_volume_iterator_one() {
    let it = BlockVolumeIterator::new(1, 1, 1);
    let mut count = 0;
    for _ in it {
        count += 1;
    }
    assert_eq!(count, 1);
}

#[test]
fn test_block_volume_iterator() {
    let volume = BlockVolume::new(BlockCoord::new(0,0,0), BlockCoord::new(10,10,10));
    let mut count = 0;
    for _ in volume.iter() {
        count += 1;
    }
    assert_eq!(count, 10*10*10);
}

#[test]
fn test_block_volume_inclusive_iterator() {
    let volume = BlockVolume::new_inclusive(BlockCoord::new(0,0,0), BlockCoord::new(10,10,10));
    let mut count = 0;
    for _ in volume.iter() {
        count += 1;
    }
    assert_eq!(count, 11*11*11);
}

#[test]
fn test_block_volume_container_iterator() {
    let volume = BlockVolume::new(BlockCoord::new(0,0,0), BlockCoord::new(10,10,10));
    let mut container = BlockVolumeContainer::new(volume);
    let mut count = 0;
    for coord in volume.iter() {
        container[coord] = Some(BlockType::Empty);
        count += 1;
    }
    assert_eq!(count, 10*10*10);
}

#[test]
fn test_block_volume_container_inclusive_iterator() {
    let volume = BlockVolume::new_inclusive(BlockCoord::new(0,0,0), BlockCoord::new(10,10,10));
    let mut container = BlockVolumeContainer::new(volume);
    let mut count = 0;
    for coord in volume.iter() {
        container[coord] = Some(BlockType::Empty);
        count += 1;
    }
    assert_eq!(count, 11*11*11);
}