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
        assert_eq!(min_time, 1.0);
    }
}
