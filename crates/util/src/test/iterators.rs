use bevy::math::IVec3;

use crate::iterators::*;

#[test]
fn test_volume_iterator() {
    let it = VolumeIterator::new(5, 6, 7);
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
    let it = VolumeIterator::new(0, 0, 0);
    let mut count = 0;
    for _ in it {
        count += 1;
    }
    assert_eq!(count, 0);
}

#[test]
fn test_volume_iterator_one() {
    let it = VolumeIterator::new(1, 1, 1);
    let mut count = 0;
    for _ in it {
        count += 1;
    }
    assert_eq!(count, 1);
}

#[test]
fn test_block_volume_iterator() {
    let volume = Volume::new(IVec3::new(0, 0, 0), IVec3::new(10, 10, 10));
    let mut count = 0;
    for _ in volume.iter() {
        count += 1;
    }
    assert_eq!(count, 10 * 10 * 10);
}

#[test]
fn test_block_volume_inclusive_iterator() {
    let volume = Volume::new_inclusive(IVec3::new(0, 0, 0), IVec3::new(10, 10, 10));
    let mut count = 0;
    for _ in volume.iter() {
        count += 1;
    }
    assert_eq!(count, 11 * 11 * 11);
}

#[test]
fn test_block_volume_container_iterator() {
    let volume = Volume::new(IVec3::new(0, 0, 0), IVec3::new(10, 10, 10));
    let mut container = VolumeContainer::new(volume);
    let mut count = 0;
    for coord in volume.iter() {
        container.set(coord, Some(1));
        count += 1;
    }
    assert_eq!(count, 10 * 10 * 10);
}

#[test]
fn test_block_volume_container_inclusive_iterator() {
    let volume = Volume::new_inclusive(IVec3::new(0, 0, 0), IVec3::new(10, 10, 10));
    let mut container = VolumeContainer::new(volume);
    let mut count = 0;
    for coord in volume.iter() {
        container.set(coord, Some(1));
        count += 1;
    }
    assert_eq!(count, 11 * 11 * 11);
}
