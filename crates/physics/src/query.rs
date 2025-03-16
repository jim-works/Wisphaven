use crate::collision::{Aabb, BlockPhysics};
use bevy::prelude::*;
use world::block::BlockCoord;
use world::level::Level;

#[derive(Copy, Clone)]
pub struct Raycast {
    pub origin: Vec3,
    pub direction: Dir3,
    pub length: f32,
}

impl Raycast {
    pub fn new(origin: Vec3, direction: Dir3, length: f32) -> Raycast {
        Self {
            origin,
            direction,
            length,
        }
    }
}

pub enum RaycastHit {
    Block(BlockCoord, RayCastHitEntity),
    Object(RayCastHitEntity),
}

pub struct RayCastHitEntity {
    pub hit_pos: Vec3,
    pub normal: util::direction::Direction,
    pub entity: Entity,
}

//todo improve this: blockcast for blocks (breseham's -> sweeping), only query entities store in chunks along the line and sweep
//todo: normal -- update use_block_entity_item after
pub fn raycast(
    ray: Raycast,
    level: &Level,
    physics_query: &Query<&BlockPhysics>,
    object_query: &Query<(Entity, &GlobalTransform, &Aabb)>,
    exclude: &[Entity],
) -> Option<RaycastHit> {
    const STEP_SIZE: f32 = 1. / 32.;
    let mut dist = 0.0;
    while dist <= ray.length {
        //test intersection
        let test_point = ray.origin + ray.direction * dist;
        //test block
        let test_block_coord = BlockCoord::from(test_point);
        if let Some(block_entity) = level.get_block_entity(test_block_coord) {
            if !exclude.contains(&block_entity) {
                if let Some(collider) = physics_query
                    .get(block_entity)
                    .ok()
                    .and_then(Aabb::from_block)
                {
                    if collider.intersects_point(test_block_coord.to_vec3(), test_point) {
                        //our point intersects the block
                        return Some(RaycastHit::Block(
                            test_block_coord,
                            RayCastHitEntity {
                                hit_pos: test_point,
                                entity: block_entity,
                                normal: util::direction::Direction::PosY,
                            },
                        ));
                    }
                }
            }
        }
        //test entity
        for (entity, tf, col) in object_query.iter() {
            if exclude.contains(&entity) {
                continue;
            }
            if col.intersects_point(tf.translation(), test_point) {
                //our point intersects an entity
                return Some(RaycastHit::Object(RayCastHitEntity {
                    hit_pos: test_point,
                    normal: util::direction::Direction::PosY,
                    entity,
                }));
            }
        }
        dist += STEP_SIZE;
    }
    None
}

pub mod blockcast_checkers {
    use crate::collision::BlockPhysics;
    use bevy::prelude::*;
    use world::block::BlockType;

    pub fn non_empty(opt_block: Option<BlockType>) -> bool {
        opt_block
            .map(|b| !matches!(b, BlockType::Empty))
            .unwrap_or(false)
    }

    pub fn empty(opt_block: Option<BlockType>) -> bool {
        opt_block
            .map(|b| matches!(b, BlockType::Empty))
            .unwrap_or(true)
    }

    pub fn solid(physics_query: &Query<&BlockPhysics>, opt_block: Option<BlockType>) -> bool {
        opt_block
            .map(|b| match b {
                BlockType::Empty => false,
                BlockType::Filled(e) => physics_query
                    .get(e)
                    .unwrap_or(&BlockPhysics::Empty)
                    .is_solid(),
            })
            .unwrap_or(false)
    }
}
