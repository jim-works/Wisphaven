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
    if let Some((block, corrected_v, min_time, Some(normal))) = time {
        let exp_time = 0.2;
        assert_eq!(block, BlockCoord::new(0, 0, 0));
        assert_eq!(corrected_v, Vec3::new(0.0, -1.0, 0.0));
        assert_eq!(min_time, exp_time);
        assert_eq!(normal, crate::util::Direction::PosY);
    }

    //upward test
    let blocks = vec![(BlockCoord::new(0, 4, 0), &BlockPhysics::Solid)];
    let time = collider.min_time_to_collision(
        blocks.into_iter(),
        Vec3::new(0.0, 2.5, 0.0),
        Vec3::new(0.0, 5.0, 0.0),
    );
    //bottom of collider is 1 unit below top of block collider
    //so d=1, t=d/v
    assert!(time.is_some());
    if let Some((block, corrected_v, min_time, Some(normal))) = time {
        let exp_time = 0.2;
        assert_eq!(block, BlockCoord::new(0, 4, 0));
        assert_eq!(corrected_v, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(min_time, exp_time);
        assert_eq!(normal, crate::util::Direction::NegY);
    }
}

#[test]
fn time_to_collision_multi_axis() {
    let collider = Collider {
        shape: Aabb::new(Vec3::splat(0.5)),
        offset: Vec3::ZERO,
    };
    let blocks = vec![
        (BlockCoord::new(0, -1, 0), &BlockPhysics::Solid),
        (BlockCoord::new(1, 0, 0), &BlockPhysics::Solid),
    ];
    let time = collider.min_time_to_collision(
        blocks.into_iter(),
        Vec3::new(0., 2.5, 0.0),
        Vec3::new(1.0, -8.0, 0.0),
    );
    //bottom of collider is 1 unit above top of block collider
    //so d=1, t=d/v
    assert!(time.is_some());
    if let Some((block, corrected_v, min_time, Some(normal))) = time {
        let exp_time = 0.25;
        assert_eq!(block, BlockCoord::new(0, -1, 0));
        assert_eq!(corrected_v, Vec3::new(0.25, -2.0, 0.0));
        assert_eq!(min_time, exp_time);
        assert_eq!(normal, crate::util::Direction::PosY);
    } else {
        assert!(false);
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
    let col = Collider {
        shape: Aabb::new(Vec3::new(1.0, 2.0, 3.0)),
        offset: Vec3::new(3.0, 2.0, 3.0),
    };
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
fn aabb_displacement_one_axis_exterior() {
    let from_aabb = Aabb::new(Vec3::splat(0.5));
    let to_aabb = Aabb::new(Vec3::splat(0.5));
    assert_eq!(
        from_aabb.axis_displacement(Vec3::new(0.0, 2.0, 0.0), to_aabb),
        Vec3::new(f32::INFINITY, 1.0, f32::INFINITY)
    );

    let from_aabb = Aabb::new(Vec3::splat(0.5));
    let to_aabb = Aabb::new(Vec3::splat(0.5));
    assert_eq!(
        from_aabb.axis_displacement(Vec3::new(0.0, -2.0, 0.0), to_aabb),
        Vec3::new(f32::INFINITY, -1.0, f32::INFINITY)
    );
}

#[test]
fn aabb_displacement_one_axis_interior() {
    //x
    let from_aabb = Aabb::new(Vec3::splat(0.5));
    let to_aabb = Aabb::new(Vec3::splat(0.5));
    assert_eq!(
        from_aabb.axis_displacement(Vec3::new(0.5, 0.0, 0.0), to_aabb),
        Vec3::ZERO
    );
    assert_eq!(
        from_aabb.axis_displacement(Vec3::new(-0.5, 0.0, 0.0), to_aabb),
        Vec3::ZERO
    );
    //y
    assert_eq!(
        from_aabb.axis_displacement(Vec3::new(0.0, 0.5, 0.0), to_aabb),
        Vec3::ZERO
    );
    assert_eq!(
        from_aabb.axis_displacement(Vec3::new(0.0, -0.5, 0.0), to_aabb),
        Vec3::ZERO
    );
    //z
    assert_eq!(
        from_aabb.axis_displacement(Vec3::new(0.0, 0.0, 0.5), to_aabb),
        Vec3::ZERO
    );
    assert_eq!(
        from_aabb.axis_displacement(Vec3::new(0.0, 0.0, -0.5), to_aabb),
        Vec3::ZERO
    );
}

#[test]
fn sweep_hit_velocity() {
    let origin = Aabb::new(Vec3::new(1.0, 2.0, 3.0));
    let other = Aabb::new(Vec3::new(6.0, 4.0, 2.0));
    let res = origin.sweep(
        Vec3::new(107.0, 306.0, 205.0),
        other,
        Vec3::new(400.0, 1200.0, 800.0),
    );
    if let Some((time, updated_v, opt_normal)) = res {
        // (100, 100, 100) units on each axis
        // (100, 100, 100) velocity on each axis
        // (1.0, 1.0, 1.0) time on each axis
        // should penetrate x the most
        assert_eq!(updated_v, Vec3::new(100.0, 300.0, 200.0));
        assert_eq!(time, 0.25);
        assert!(opt_normal.is_some());
    } else {
        assert!(false);
    }
}

#[test]
fn sweep_hit_one_axis() {
    let origin = Aabb::new(Vec3::new(1.0, 2.0, 3.0));
    let other = Aabb::new(Vec3::new(100.0, 4.0, 100.0));
    let res = origin.sweep(
        Vec3::new(0.0, -16.0, 0.0),
        other,
        Vec3::new(1.0, -20.0, -2.0),
    );
    if let Some((time, updated_v, opt_normal)) = res {
        //penetrates on y axis the most
        if let Some(normal) = opt_normal {
            assert_eq!(updated_v, Vec3::new(0.5, -10.0, -1.0));
            assert_eq!(time, 0.5);
            assert_eq!(normal, crate::util::Direction::PosY);
        }
    } else {
        assert!(false);
    }
}

#[test]
fn sweep_hit_normal() {
    let origin = Aabb::new(Vec3::new(1.0, 2.0, 3.0));
    let other = Aabb::new(Vec3::new(6.0, 4.0, 2.0));
    let res = origin.sweep(
        Vec3::new(107.0, 306.0, 205.0),
        other,
        Vec3::new(400.5, 1200.0, 800.0),
    );
    if let Some((_, _, Some(normal))) = res {
        assert_eq!(normal, crate::util::Direction::NegX); //penetrate deepest on x axis
    } else {
        assert!(false);
    }
}

#[test]
fn sweep_hit_inside() {
    let origin = Aabb::new(Vec3::new(1.0, 2.0, 3.0));
    let other = Aabb::new(Vec3::new(6.0, 4.0, 2.0));
    let res = origin.sweep(Vec3::ZERO, other, Vec3::new(1.0, 2.0, 3.0));
    if let Some((time, updated_v, None)) = res {
        assert_eq!(time, 0.0);
        assert_eq!(updated_v, Vec3::ZERO);
    } else {
        assert!(false);
    }
}

#[test]
fn sweep_hit_miss() {
    let origin = Aabb::new(Vec3::new(1.0, 2.0, 3.0));
    let other = Aabb::new(Vec3::new(6.0, 4.0, 2.0));
    let res = origin.sweep(Vec3::splat(20.0), other, Vec3::new(1.0, 2.0, 3.0));
    assert!(res.is_none());
}

#[test]
fn far_collision_false_positive_patch() {
    let col = Collider {
        shape: Aabb::new(Vec3::new(0.4, 0.8, 0.4)),
        offset: Vec3::new(0., 0.8, 0.),
    };
    let offset = Vec3::new(0.0,12.0,10.0);
    let v = Vec3::new(0.0,-0.0075,0.0);
    let block_coord = BlockCoord::new(-4,11,12);
    let res = col.min_time_to_collision(std::iter::once((block_coord, &BlockPhysics::Solid)), offset, v);
    println!("{:?}", res);
    assert!(res.is_none());
}