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
    if let Some((block, corrected_v, min_time, normal)) = time {
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
    if let Some((block, corrected_v, min_time, normal)) = time {
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
    if let Some((block, corrected_v, min_time, normal)) = time {
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
        Vec3::new(0.49, 1.9, 4.4),
        Aabb::new(Vec3::new(1.0, 2.0, 3.0))
    ));
}

#[test]
fn aabb_intersects_false() {
    assert!(!Aabb::intersects(
        Aabb::new(Vec3::new(0.5, 1.0, 1.5)),
        Vec3::new(2.5, 5.0, 7.5),
        Aabb::new(Vec3::new(1.0, 2.0, 3.0))
    ));
    assert!(!Aabb::intersects(
        Aabb::new(Vec3::new(0.5, 1.0, 1.5)),
        Vec3::new(0.5, 2.0, 4.5),
        Aabb::new(Vec3::new(1.0, 2.0, 3.0))
    ));
}

#[test]
fn aabb_overlap_displacement_one_axis_interior() {
    //x
    let from_aabb = Aabb::new(Vec3::splat(0.5));
    let to_aabb = Aabb::new(Vec3::splat(0.5));
    assert_eq!(
        from_aabb.overlapping_displacement(Vec3::new(0.5, 0.0, 0.0), to_aabb),
        Vec3::new(-0.5, 1.0, 1.0)
    );
    assert_eq!(
        from_aabb.overlapping_displacement(Vec3::new(-0.5, 0.0, 0.0), to_aabb),
        Vec3::new(0.5, 1.0, 1.0)
    );
    //y
    assert_eq!(
        from_aabb.overlapping_displacement(Vec3::new(0.0, 0.5, 0.0), to_aabb),
        Vec3::new(1.0, -0.5, 1.0)
    );
    assert_eq!(
        from_aabb.overlapping_displacement(Vec3::new(0.0, -0.5, 0.0), to_aabb),
        Vec3::new(1.0, 0.5, 1.0)
    );
    //z
    assert_eq!(
        from_aabb.overlapping_displacement(Vec3::new(0.0, 0.0, 0.5), to_aabb),
        Vec3::new(1.0, 1.0, -0.5)
    );
    assert_eq!(
        from_aabb.overlapping_displacement(Vec3::new(0.0, 0.0, -0.5), to_aabb),
        Vec3::new(1.0, 1.0, 0.5)
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
    if let Some((time, updated_v, _normal)) = res {
        // (100, 100, 100) units on each axis
        // (100, 100, 100) velocity on each axis
        // (1.0, 1.0, 1.0) time on each axis
        // should penetrate x the most
        assert_eq!(updated_v, Vec3::new(100.0, 300.0, 200.0));
        assert_eq!(time, 0.25);
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
    if let Some((time, updated_v, normal)) = res {
        //penetrates on y axis the most
        assert_eq!(updated_v, Vec3::new(0.5, -10.0, -1.0));
        assert_eq!(time, 0.5);
        assert_eq!(normal, crate::util::Direction::PosY);
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
    if let Some((_, _, normal)) = res {
        assert_eq!(normal, crate::util::Direction::NegZ); //penetrate shallowest on z axis
    } else {
        assert!(false);
    }
}

#[test]
fn sweep_hit_inside() {
    let origin = Aabb::new(Vec3::new(1.0, 2.0, 3.0));
    let other = Aabb::new(Vec3::new(1.0, 4.0, 2.0));
    let res = origin.sweep(Vec3::new(1.0,0.,0.), other, Vec3::new(1.0, 2.0, 3.0));
    if let Some((time, updated_v, normal)) = res {
        assert_eq!(time, 0.0);
        assert_eq!(updated_v, Vec3::new(-1.0,0.0,0.0));
        assert_eq!(normal, crate::util::Direction::NegX);
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
    let offset = Vec3::new(0.0, 12.0, 10.0);
    let v = Vec3::new(0.0, -0.0075, 0.0);
    let block_coord = BlockCoord::new(-4, 11, 12);
    let res = col.min_time_to_collision(
        std::iter::once((block_coord, &BlockPhysics::Solid)),
        offset,
        v,
    );
    println!("{:?}", res);
    assert!(res.is_none());
}

#[test]
fn resolve_collision_inside_block() {
    let aabb = Aabb::new(Vec3::new(0.4, 0.8, 0.4));
    let block_aabb = Aabb::new(Vec3::splat(0.5));
    let offset = Vec3::new(-0.5, -0.5, -0.8999996);
    let correction_opt = aabb.sweep(offset, block_aabb, Vec3::ZERO);
    assert!(correction_opt.is_some());
    let (_time, correction, dir) = correction_opt.unwrap();
    //test collision normal
    assert_eq!(dir, crate::util::Direction::PosZ);
    let corrected_offset = offset + correction;
    println!("d: {:?}", corrected_offset.z+aabb.extents.z+block_aabb.extents.z);
    let corrected_sweep = aabb.sweep(corrected_offset, block_aabb, Vec3::ZERO);
    assert!(corrected_sweep.is_none());
}

#[test]
fn resolve_collision_inside_block_penetration() {
    let aabb = Aabb::new(Vec3::new(0.4, 0.8, 0.4));
    let block_aabb = Aabb::new(Vec3::splat(0.5));
    let offset = Vec3::new(-0.5, -0.5, -0.8999996);
    let correction_opt = aabb.sweep(offset, block_aabb, Vec3::ZERO);
    assert!(correction_opt.is_some());
    let (_time, correction, dir) = correction_opt.unwrap();
    //test collision normal
    assert_eq!(dir, crate::util::Direction::PosZ);
    let corrected_offset = offset + correction;
    println!("d: {:?}", corrected_offset.z+aabb.extents.z+block_aabb.extents.z);
    let corrected_sweep = aabb.sweep(corrected_offset, block_aabb, Vec3::ZERO);
    assert!(corrected_sweep.is_none());
}
/*
2023-12-19T00:10:11.858386Z  INFO wisphaven::physics::collision: tf: Vec3(-11.594771, 13.0, 5.1022277), time_remainig: 0.13549556

2023-12-19T00:10:11.858501Z  INFO wisphaven::physics::collision: tf: Vec3(-11.6, 13.0, 5.103908), time_remainig: 0.0

2023-12-19T00:10:11.858506Z  WARN wisphaven::physics::collision: inside block!
2023-12-19T00:10:11.858509Z  INFO wisphaven::physics::collision: Collider { shape: Aabb { extents: Vec3(0.4, 0.8, 0.4) }, offset: Vec3(0.0, 0.8, 0.0) }
tf: Vec3(-11.6, 13.0, 5.103908)
BlockCoord { x: -13, y: 13, z: 5 }
v: Vec3(0.0, 0.0, 0.012402501)
*/

// #[test]
// fn clip_inside_block_false_positive_patch() {
//     let col = Collider {
//         shape: Aabb::new(Vec3::new(0.4, 0.8, 0.4)),
//         offset: Vec3::new(0., 0.8, 0.),
//     };
//     let p = Vec3::new(0.45, 0.0, 0.6);
//     let v = Vec3::ZERO; //Vec3::new(-1.0,0.0,1.0);
//     let blocks = [
//         (BlockCoord::new(-1, 0, 0), &BlockPhysics::Solid),
//         (BlockCoord::new(0, 0, 1), &BlockPhysics::Solid),
//     ]
//     .into_iter();
//     let res = col.min_time_to_collision(blocks, p, v);
//     println!("{:?}", res);
//     assert!(res.is_some());
//     if let Some((_coord, v, _time, normal)) = res {
//         // assert!(crate::util::Direction::);
//         assert_eq!(v, Vec3::ZERO);
//     }
// }
