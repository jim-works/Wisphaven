use std::assert_matches::assert_matches;

use crate::{physics::collision::*, world::*};
use bevy::prelude::*;

#[test]
fn test_time_to_collision() {
    let collider = Collider {
        shape: Aabb::new(Vec3::splat(0.5)),
        offset: Vec3::ZERO,
    };
    let blocks = vec![(BlockCoord::new(0, 0, 0), &BlockPhysics::Solid)];
    let time = collider.min_time_to_collision(
        blocks.into_iter(),
        Vec3::new(0.0, 2.5, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
    );
    assert!(time.is_some());
    //bottom of collider is 1 unit above top of block collider
    //at v = -1, should be 1
    if let Some((block, time_vec, min_time)) = time {
        assert_eq!(block, BlockCoord::new(0,0,0));
        assert_eq!(time_vec, Vec3::new(f32::INFINITY, 1.0, f32::INFINITY));
        assert_eq!(min_time, 1.0);
    }
}

#[test]
fn test_from_block() {
    assert_matches!(Collider::from_block(&BlockPhysics::Empty), None);
    assert_matches!(Collider::from_block(&BlockPhysics::Solid), Some(_));
    let col = Collider { shape: Aabb::new(Vec3::new(1.0,2.0,3.0)), offset: Vec3::new(3.0,2.0,3.0) };
    assert_matches!(Collider::from_block(&BlockPhysics::Aabb(col)), Some(_));
}

#[test]
fn aabb_distance() {
    let from_aabb = Aabb::new(Vec3::splat(0.5));
    let to_aabb = Aabb::new(Vec3::splat(0.5));
    assert_eq!(from_aabb.axis_distance(Vec3::new(0.0,2.0,0.0), to_aabb), Vec3::new(-1.0,1.0,-1.0));
}