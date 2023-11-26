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

#[derive(Component, Default)]
pub struct Collider {
    pub shape: ColliderShape,
    pub offset: Vec3,
}

impl Collider {
    fn resolve(
        &self,
        potential_overlap: BlockVolume,
        p: &mut Vec3,
        v: &mut Vec3,
        level: &Level,
        block_physics: &Query<&BlockPhysics>,
        mut shape_resolver: impl FnMut(
            &ColliderShape,
            BlockCoord,
            Option<&BlockPhysics>,
            &mut Vec3,
            &mut Vec3,
        ) -> bool,
    ) -> bool {
        let mut collision = false;
        let mut relative_position = *p + self.offset;
        for coord in potential_overlap.iter() {
            collision |= shape_resolver(
                &self.shape,
                coord,
                level
                    .get_block_entity(coord)
                    .and_then(|b| block_physics.get(b).ok()),
                &mut relative_position,
                v,
            );
        }
        *p = relative_position - self.offset;
        collision
    }

    fn get_potential_overlap(
        &self,
        p: Vec3,
        shape_overlap_provider: impl Fn(&ColliderShape, Vec3) -> BlockVolume,
    ) -> BlockVolume {
        shape_overlap_provider(&self.shape, p + self.offset)
    }
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Box(Aabb),
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Box(Aabb {
            extents: Vec3::new(0.5, 0.5, 0.5),
        })
    }
}

impl ColliderShape {
    const FACE_SHRINK_MULT: f32 = 0.99; //shrinks each face on the box collider by this proportion to avoid conflicting collisions against walls
    
    //self.position + delta = aabb.position
    pub fn intersects_aabb(&self, delta: Vec3, aabb: Aabb) -> bool {
        let min_corner = delta - aabb.extents;
        let max_corner = delta + aabb.extents;
        match self {
            ColliderShape::Box(my_aabb) => {
                (-my_aabb.extents.x <= max_corner.x
                    && -my_aabb.extents.y <= max_corner.y
                    && -my_aabb.extents.z <= max_corner.z)
                    && (my_aabb.extents.x >= min_corner.x
                        && my_aabb.extents.y >= min_corner.y
                        && my_aabb.extents.z >= min_corner.z)
            }
        }
    }

    fn resolve(
        &self,
        p: &mut Vec3,
        v: &mut Vec3,
        level: &Level,
        block_physics: &Query<&BlockPhysics>,
        potential_overlap: BlockVolume,
        mut resolver: impl FnMut(
            &ColliderShape,
            BlockCoord,
            Option<&BlockPhysics>,
            &mut Vec3,
            &mut Vec3,
        ) -> bool,
    ) -> bool {
        let mut collision = false;
        for coord in potential_overlap.iter() {
            collision |= resolver(
                self,
                coord,
                level
                    .get_block_entity(coord)
                    .and_then(|b| block_physics.get(b).ok()),
                p,
                v,
            );
        }
        collision
    }

    pub fn potential_overlapping_blocks_neg_y(&self, delta: Vec3) -> BlockVolume {
        match self {
            ColliderShape::Box(aabb) => BlockVolume::from_aabb(
                Aabb {
                    extents: Vec3::new(aabb.extents.x, 0.0, aabb.extents.z)*Self::FACE_SHRINK_MULT,
                },
                delta - Vec3::new(0., aabb.extents.y, 0.),
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
    ) -> bool {
        match self {
            ColliderShape::Box(aabb) => match block_physics {
                Some(BlockPhysics::Solid) => {
                    velocity.y = 0.0;
                    position.y = (coord.y + 1) as f32 + aabb.extents.y;
                    true
                }
                Some(BlockPhysics::BottomSlab(_height)) => {
                    todo!()
                }
                Some(BlockPhysics::Empty) | None => false,
            },
        }
    }

    pub fn potential_overlapping_blocks_pos_y(&self, delta: Vec3) -> BlockVolume {
        match self {
            ColliderShape::Box(aabb) => BlockVolume::from_aabb(
                Aabb {
                    extents: Vec3::new(aabb.extents.x, 0.0, aabb.extents.z)*Self::FACE_SHRINK_MULT,
                },
                delta + Vec3::new(0., aabb.extents.y, 0.),
            ),
        }
    }

    //assumes block is in the set returned by potential overlapping blocks
    fn resolve_terrain_collision_pos_y(
        &self,
        coord: BlockCoord,
        block_physics: Option<&BlockPhysics>,
        position: &mut Vec3,
        velocity: &mut Vec3,
    ) -> bool {
        match self {
            ColliderShape::Box(aabb) => match block_physics {
                Some(BlockPhysics::Solid) => {
                    velocity.y = 0.0;
                    position.y = coord.y as f32 - aabb.extents.y;
                    true
                }
                Some(BlockPhysics::BottomSlab(_height)) => {
                    todo!()
                }
                Some(BlockPhysics::Empty) | None => false,
            },
        }
    }

    pub fn potential_overlapping_blocks_pos_x(&self, delta: Vec3) -> BlockVolume {
        match self {
            ColliderShape::Box(aabb) => BlockVolume::from_aabb(
                Aabb {
                    extents: Vec3::new(0.0, aabb.extents.y, aabb.extents.z)*Self::FACE_SHRINK_MULT,
                },
                delta + Vec3::new(aabb.extents.x, 0., 0.),
            ),
        }
    }

    //assumes block is in the set returned by potential overlapping blocks
    fn resolve_terrain_collision_pos_x(
        &self,
        coord: BlockCoord,
        block_physics: Option<&BlockPhysics>,
        position: &mut Vec3,
        velocity: &mut Vec3,
    ) -> bool {
        match self {
            ColliderShape::Box(aabb) => match block_physics {
                Some(BlockPhysics::Solid) => {
                    velocity.x = 0.0;
                    position.x = coord.x as f32 - aabb.extents.x;
                    true
                }
                Some(BlockPhysics::BottomSlab(_height)) => {
                    todo!()
                }
                Some(BlockPhysics::Empty) | None => false,
            },
        }
    }

    pub fn potential_overlapping_blocks_neg_x(&self, delta: Vec3) -> BlockVolume {
        match self {
            ColliderShape::Box(aabb) => BlockVolume::from_aabb(
                Aabb {
                    extents: Vec3::new(0.0, aabb.extents.y, aabb.extents.z)*Self::FACE_SHRINK_MULT,
                },
                delta - Vec3::new(aabb.extents.x, 0., 0.),
            ),
        }
    }

    //assumes block is in the set returned by potential overlapping blocks
    fn resolve_terrain_collision_neg_x(
        &self,
        coord: BlockCoord,
        block_physics: Option<&BlockPhysics>,
        position: &mut Vec3,
        velocity: &mut Vec3,
    ) -> bool {
        match self {
            ColliderShape::Box(aabb) => match block_physics {
                Some(BlockPhysics::Solid) => {
                    velocity.x = 0.0;
                    position.x = (coord.x + 1) as f32 + aabb.extents.x;
                    true
                }
                Some(BlockPhysics::BottomSlab(_height)) => {
                    todo!()
                }
                Some(BlockPhysics::Empty) | None => false,
            },
        }
    }

    pub fn potential_overlapping_blocks_pos_z(&self, delta: Vec3) -> BlockVolume {
        match self {
            ColliderShape::Box(aabb) => BlockVolume::from_aabb(
                Aabb {
                    extents: Vec3::new(aabb.extents.x, aabb.extents.y, 0.0)*Self::FACE_SHRINK_MULT,
                },
                delta + Vec3::new(0., 0., aabb.extents.z),
            ),
        }
    }

    //assumes block is in the set returned by potential overlapping blocks
    fn resolve_terrain_collision_pos_z(
        &self,
        coord: BlockCoord,
        block_physics: Option<&BlockPhysics>,
        position: &mut Vec3,
        velocity: &mut Vec3,
    ) -> bool {
        match self {
            ColliderShape::Box(aabb) => match block_physics {
                Some(BlockPhysics::Solid) => {
                    velocity.z = 0.0;
                    position.z = coord.z as f32 - aabb.extents.z;
                    true
                }
                Some(BlockPhysics::BottomSlab(_height)) => {
                    todo!()
                }
                Some(BlockPhysics::Empty) | None => false,
            },
        }
    }

    pub fn potential_overlapping_blocks_neg_z(&self, delta: Vec3) -> BlockVolume {
        match self {
            ColliderShape::Box(aabb) => BlockVolume::from_aabb(
                Aabb {
                    extents: Vec3::new(aabb.extents.x, aabb.extents.y, 0.0)*Self::FACE_SHRINK_MULT,
                },
                delta - Vec3::new(0., 0., aabb.extents.z),
            ),
        }
    }

    //assumes block is in the set returned by potential overlapping blocks
    fn resolve_terrain_collision_neg_z(
        &self,
        coord: BlockCoord,
        block_physics: Option<&BlockPhysics>,
        position: &mut Vec3,
        velocity: &mut Vec3,
    ) -> bool {
        match self {
            ColliderShape::Box(aabb) => match block_physics {
                Some(BlockPhysics::Solid) => {
                    velocity.z = 0.0;
                    position.z = (coord.z + 1) as f32 + aabb.extents.z;
                    true
                }
                Some(BlockPhysics::BottomSlab(_height)) => {
                    todo!()
                }
                Some(BlockPhysics::Empty) | None => false,
            },
        }
    }
}

//axis-aligned bounding box
//not a collider atm
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub extents: Vec3,
}

#[derive(Component, Copy, Clone)]
pub struct IgnoreTerrainCollision;

fn resolve_terrain_collisions(
    mut objects: Query<(&mut Transform, &mut Velocity, &Collider), Without<IgnoreTerrainCollision>>,
    block_physics: Query<&BlockPhysics>,
    level: Res<Level>,
) {
    for (mut tf, mut v, col) in objects.iter_mut() {
        let mut corrected_position = tf.translation;
        let mut corrected_velocity = v.0;

        col.resolve(
            col.get_potential_overlap(
                corrected_position,
                ColliderShape::potential_overlapping_blocks_pos_y,
            ),
            &mut corrected_position,
            &mut corrected_velocity,
            &level,
            &block_physics,
            ColliderShape::resolve_terrain_collision_pos_y,
        );

        col.resolve(
            col.get_potential_overlap(
                corrected_position,
                ColliderShape::potential_overlapping_blocks_neg_y,
            ),
            &mut corrected_position,
            &mut corrected_velocity,
            &level,
            &block_physics,
            ColliderShape::resolve_terrain_collision_neg_y,
        );

        col.resolve(
            col.get_potential_overlap(
                corrected_position,
                ColliderShape::potential_overlapping_blocks_pos_x,
            ),
            &mut corrected_position,
            &mut corrected_velocity,
            &level,
            &block_physics,
            ColliderShape::resolve_terrain_collision_pos_x,
        );

        col.resolve(
            col.get_potential_overlap(
                corrected_position,
                ColliderShape::potential_overlapping_blocks_neg_x,
            ),
            &mut corrected_position,
            &mut corrected_velocity,
            &level,
            &block_physics,
            ColliderShape::resolve_terrain_collision_neg_x,
        );

        col.resolve(
            col.get_potential_overlap(
                corrected_position,
                ColliderShape::potential_overlapping_blocks_pos_z,
            ),
            &mut corrected_position,
            &mut corrected_velocity,
            &level,
            &block_physics,
            ColliderShape::resolve_terrain_collision_pos_z,
        );

        col.resolve(
            col.get_potential_overlap(
                corrected_position,
                ColliderShape::potential_overlapping_blocks_neg_z,
            ),
            &mut corrected_position,
            &mut corrected_velocity,
            &level,
            &block_physics,
            ColliderShape::resolve_terrain_collision_neg_z,
        );

        tf.translation = corrected_position;
        v.0 = corrected_velocity;
    }
}
