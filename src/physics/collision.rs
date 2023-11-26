use bevy::prelude::*;

use crate::{
    util::iterators::BlockVolume,
    world::{BlockCoord, BlockPhysics, Level},
};

use super::{movement::*, PhysicsSystemSet};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            resolve_terrain_collisions.in_set(PhysicsSystemSet::CollisionResolution),
        );
    }
}

//cylinder
#[derive(Component, Clone)]
pub enum Collider {
    Box(Aabb),
}

impl Default for Collider {
    fn default() -> Self {
        Self::Box(Aabb {
            extents: Vec3::new(0.5, 0.5, 0.5),
        })
    }
}

impl Collider {
    //self.position + delta = aabb.position
    pub fn intersects_aabb(&self, delta: Vec3, aabb: Aabb) -> bool {
        let min_corner = delta - aabb.extents;
        let max_corner = delta + aabb.extents;
        match self {
            Collider::Box(my_aabb) => {
                (-my_aabb.extents.x <= max_corner.x
                    && -my_aabb.extents.y <= max_corner.y
                    && -my_aabb.extents.z <= max_corner.z)
                    && (my_aabb.extents.x >= min_corner.x
                        && my_aabb.extents.y >= min_corner.y
                        && my_aabb.extents.z >= min_corner.z)
            }
        }
    }

    pub fn potential_overlapping_blocks_neg_y(&self, delta: Vec3) -> BlockVolume {
        match self {
            Collider::Box(aabb) => BlockVolume::from_aabb(
                Aabb {
                    extents: Vec3::new(aabb.extents.x, 0.0, aabb.extents.z),
                },
                delta - aabb.extents.y,
            ),
        }
    }

    //assumes block is in the set returned by potential overlapping blocks
    fn resolve_terrain_collision_neg_y(
        &self,
        coord: BlockCoord,
        block_physics: Option<&BlockPhysics>,
        position: &mut Vec3,
        velocity: &mut Vec3,
    ) {
        match self {
            Collider::Box(aabb) => match block_physics {
                Some(BlockPhysics::Solid) => {
                    velocity.y = 0.0;
                    position.y = (coord.y + 1) as f32 + aabb.extents.y;
                }
                Some(BlockPhysics::BottomSlab(_height)) => {
                    todo!()
                }
                Some(BlockPhysics::Empty) | None => {}
            },
        }
    }
}

//axis-aligned bounding box
//not a collider atm
#[derive(Clone, Copy)]
pub struct Aabb {
    pub extents: Vec3,
}

#[derive(Component, Copy, Clone)]
pub struct IgnoreTerrainCollision;

fn resolve_terrain_collisions(
    mut objects: Query<
        (&GlobalTransform, &mut Transform, &mut Velocity, &Collider),
        Without<IgnoreTerrainCollision>,
    >,
    block_physics: Query<&BlockPhysics>,
    level: Res<Level>,
) {
    for (gtf, mut tf, mut v, col) in objects.iter_mut() {
        let mut corrected_position = tf.translation;
        let mut corrected_velocity = v.0;
        let potential_overlap_neg_y = col.potential_overlapping_blocks_neg_y(gtf.translation());
        if potential_overlap_neg_y.volume() != 4 {
            info!("volume: {}", potential_overlap_neg_y.volume());
        }
        for coord in potential_overlap_neg_y.iter() {
            col.resolve_terrain_collision_neg_y(
                coord,
                level
                    .get_block_entity(coord)
                    .and_then(|b| block_physics.get(b).ok()),
                &mut corrected_position,
                &mut corrected_velocity,
            );
        }
        tf.translation = corrected_position;
        v.0 = corrected_velocity;
    }
}
