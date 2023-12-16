use bevy::prelude::*;

use crate::{
    physics::collision::Collider,
    world::{BlockCoord, BlockPhysics, Level},
};

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub length: f32,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3, length: f32) -> Ray {
        Self {
            origin,
            direction,
            length,
        }
    }
}

pub enum RaycastHit {
    Block(BlockCoord, Vec3, Entity),
    Object(Vec3, Entity),
}

pub fn raycast(
    ray: Ray,
    level: &Level,
    physics_query: &Query<&BlockPhysics>,
) -> Option<RaycastHit> {
    const STEP_SIZE: f32 = 1. / 32.;
    let mut dist = 0.0;
    while dist <= ray.length {
        //test intersection
        let test_point = ray.origin + ray.direction * dist;
        let test_block_coord = BlockCoord::from(test_point);
        if let Some(block_entity) = level.get_block_entity(test_block_coord) {
            if let Some(collider) = physics_query.get(block_entity).ok()
                .and_then(|physics| Collider::from_block(physics))
            {
                if collider.intersects_point(test_point.fract()) {
                    //our point intersects the block
                    return Some(RaycastHit::Block(
                        test_block_coord,
                        test_point,
                        block_entity,
                    ));
                }
            }
        }
        dist += STEP_SIZE;
    }
    return None;
}
