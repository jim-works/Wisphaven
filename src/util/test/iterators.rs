use bevy::math::Vec3;

use crate::{util::iterators::*, world::*, physics::collision::Aabb};

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
    let volume = BlockVolume::new(BlockCoord::new(0, 0, 0), BlockCoord::new(10, 10, 10));
    let mut count = 0;
    for _ in volume.iter() {
        count += 1;
    }
    assert_eq!(count, 10 * 10 * 10);
}

#[test]
fn test_block_volume_inclusive_iterator() {
    let volume = BlockVolume::new_inclusive(BlockCoord::new(0, 0, 0), BlockCoord::new(10, 10, 10));
    let mut count = 0;
    for _ in volume.iter() {
        count += 1;
    }
    assert_eq!(count, 11 * 11 * 11);
}

#[test]
fn test_block_volume_container_iterator() {
    let volume = BlockVolume::new(BlockCoord::new(0, 0, 0), BlockCoord::new(10, 10, 10));
    let mut container = VolumeContainer::new(volume);
    let mut count = 0;
    for coord in volume.iter() {
        container[coord] = Some(BlockType::Empty);
        count += 1;
    }
    assert_eq!(count, 10 * 10 * 10);
}

#[test]
fn test_block_volume_container_inclusive_iterator() {
    let volume = BlockVolume::new_inclusive(BlockCoord::new(0, 0, 0), BlockCoord::new(10, 10, 10));
    let mut container = VolumeContainer::new(volume);
    let mut count = 0;
    for coord in volume.iter() {
        container[coord] = Some(BlockType::Empty);
        count += 1;
    }
    assert_eq!(count, 11 * 11 * 11);
}

#[test]
fn test_aabb_to_block_volume_inclusive() {
    let aabb = Aabb { size: Vec3::new(0.8,1.5,0.8), offset: Vec3::new(0.4,0.75,0.4) };
    let volume = BlockVolume::from_aabb(aabb, Vec3::new(-0.8,-0.8,-0.8));
    //since edges are on edges of cells, it should include those cells
    //aka: 2x3x2 = 12
    assert_eq!(volume.volume(), 12);
    assert_eq!(volume.max_corner, BlockCoord::new(1,2,1));
    assert_eq!(volume.min_corner, BlockCoord::new(-1,-1,-1));
}

#[test]
fn test_aabb_to_block_volume() {
    let aabb = Aabb { size: Vec3::new(0.8,1.5,0.8), offset: Vec3::new(-0.4,-0.75,-0.4) };
    let volume = BlockVolume::from_aabb(aabb, Vec3::new(0.5,0.8,0.5));
    //should be fully enclosed in a 1x2x1 column
    assert_eq!(volume.volume(), 2);
    assert_eq!(volume.max_corner, BlockCoord::new(1,2,1));
    assert_eq!(volume.min_corner, BlockCoord::new(0,0,0));
}