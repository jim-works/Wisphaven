use std::assert_matches::assert_matches;

use crate::{physics::collision::*, world::*};
use bevy::prelude::*;

#[test]
fn time_to_collision_single_axis() {
    let collider = Collider {
        shape: Aabb::new(Vec3::splat(0.5)),
        offset: Vec3::ZERO,
    };
    let blocks = vec![(BlockCoord::new(0, 0, 0), &BlockPhysics::Solid)];
    let time = collider.min_time_to_collision(
        blocks.into_iter(),
        Vec3::new(0.0, 2.5, 0.0),
        Vec3::new(0.0, -5.0, 0.0),
    );
    //bottom of collider is 1 unit above top of block collider
    //so d=1, t=d/v
    assert!(time.is_some());
    if let Some((block, time_vec, min_time)) = time {
        let exp_time = 0.2;
        assert_eq!(block, BlockCoord::new(0,0,0));
        assert_eq!(time_vec, Vec3::new(f32::INFINITY, exp_time, f32::INFINITY));
        assert_eq!(min_time, exp_time);
    }

    //upward test
    let blocks = vec![(BlockCoord::new(0, 4, 0), &BlockPhysics::Solid)];
    let time = collider.min_time_to_collision(
        blocks.into_iter(),
        Vec3::new(0.0, 2.5, 0.0),
        Vec3::new(0.0, 5.0, 0.0),
    );
    //bottom of collider is 1 unit above top of block collider
    //so d=1, t=d/v
    assert!(time.is_some());
    if let Some((block, time_vec, min_time)) = time {
        let exp_time = 0.2;
        assert_eq!(block, BlockCoord::new(0,4,0));
        assert_eq!(time_vec, Vec3::new(f32::INFINITY, exp_time, f32::INFINITY));
        assert_eq!(min_time, exp_time);
    }
}

#[test]
fn fail_time_to_impossible_collision() {
    let collider = Collider {
        shape: Aabb::new(Vec3::splat(0.5)),
        offset: Vec3::ZERO,
    };
    let blocks = vec![(BlockCoord::new(0, 0, 0), &BlockPhysics::Solid)];
    let time = collider.min_time_to_collision(
        blocks.into_iter(),
        Vec3::new(0.0, 2.5, 0.0),
        Vec3::new(0.0, 5.0, 0.0),
    );
    //velocity is up, so should never come into contact with block below
    assert!(time.is_none());
}

#[test]
fn from_block() {
    assert_matches!(Collider::from_block(&BlockPhysics::Empty), None);
    assert_matches!(Collider::from_block(&BlockPhysics::Solid), Some(_));
    let col = Collider { shape: Aabb::new(Vec3::new(1.0,2.0,3.0)), offset: Vec3::new(3.0,2.0,3.0) };
    assert_matches!(Collider::from_block(&BlockPhysics::Aabb(col)), Some(_));
}

#[test]
fn aabb_intersects_true() {
    assert!(Aabb::intersects(
        Aabb::new(Vec3::splat(0.5)),
        Vec3::ZERO,
        Aabb::new(Vec3::splat(0.5))
    ));
    assert!(Aabb::intersects(
        Aabb::new(Vec3::new(0.5, 1.0, 1.5)),
        Vec3::new(0.5, 2.0, 4.5),
        Aabb::new(Vec3::new(1.0, 2.0, 3.0))
    ))
}

#[test]
fn aabb_intersects_false() {
    assert!(!Aabb::intersects(
        Aabb::new(Vec3::new(0.5, 1.0, 1.5)),
        Vec3::new(2.5, 5.0, 7.5),
        Aabb::new(Vec3::new(1.0, 2.0, 3.0))
    ))
}

#[test]
fn aabb_distance_one_axis_exterior() {
    let from_aabb = Aabb::new(Vec3::splat(0.5));
    let to_aabb = Aabb::new(Vec3::splat(0.5));
    assert_eq!(from_aabb.axis_distance(Vec3::new(0.0,2.0,0.0), to_aabb), Vec3::new(f32::INFINITY,1.0,f32::INFINITY));

    let from_aabb = Aabb::new(Vec3::splat(0.5));
    let to_aabb = Aabb::new(Vec3::splat(0.5));
    assert_eq!(from_aabb.axis_distance(Vec3::new(0.0,-2.0,0.0), to_aabb), Vec3::new(f32::INFINITY,1.0,f32::INFINITY));
}

#[test]
fn aabb_distance_one_axis_interior() {
    //x
    let from_aabb = Aabb::new(Vec3::splat(0.5));
    let to_aabb = Aabb::new(Vec3::splat(0.5));
    assert_eq!(from_aabb.axis_distance(Vec3::new(0.5,0.0,0.0), to_aabb), Vec3::new(-0.5,-1.0,-1.0));
    assert_eq!(from_aabb.axis_distance(Vec3::new(-0.5,0.0,0.0), to_aabb), Vec3::new(-0.5,-1.0,-1.0));
    //y
    assert_eq!(from_aabb.axis_distance(Vec3::new(0.0,0.5,0.0), to_aabb), Vec3::new(-1.0,-0.5,-1.0));
    assert_eq!(from_aabb.axis_distance(Vec3::new(0.0,-0.5,0.0), to_aabb), Vec3::new(-1.0,-0.5,-1.0));
    //z
    assert_eq!(from_aabb.axis_distance(Vec3::new(0.0,0.0,0.5), to_aabb), Vec3::new(-1.0,-1.0,-0.5));
    assert_eq!(from_aabb.axis_distance(Vec3::new(0.0,0.0,-0.5), to_aabb), Vec3::new(-1.0,-1.0,-0.5));
}