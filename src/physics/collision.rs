use bevy::prelude::*;

use crate::{
    util::{iterators::BlockVolume, DirectionFlags},
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

#[derive(Component)]
pub struct Friction(f32);

impl Default for Friction {
    fn default() -> Self {
        Self(0.005)
    }
}

#[derive(Component, Default)]
pub struct Collider {
    pub shape: ColliderShape,
    pub offset: Vec3,
}

#[derive(Component, Default)]
pub struct CollidingDirections(pub DirectionFlags);

impl Collider {
    fn resolve(
        &self,
        potential_overlap: BlockVolume,
        p: &mut Vec3,
        v: &mut Vec3,
        a: &mut Vec3,
        normal: Vec3,
        friction: Option<&Friction>,
        level: &Level,
        block_query: &Query<(&BlockPhysics, Option<&Friction>)>,
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
        let mut block_sum_frictions = 0.;
        let mut frictions = 0;
        for coord in potential_overlap.iter() {
            let (physics, block_friction) = level
                .get_block_entity(coord)
                .and_then(|b| block_query.get(b).ok())
                .map(|(phys, opt_fric)| (Some(phys), opt_fric))
                .unwrap_or((None, None));
            let collides_with_block =
                shape_resolver(&self.shape, coord, physics, &mut relative_position, v);
            collision |= collides_with_block;
            match (friction, block_friction, collides_with_block) {
                (Some(_), Some(coeff), true) => {
                    frictions += 1;
                    block_sum_frictions += coeff.0;
                }
                _ => {}
            }
        }
        *p = relative_position - self.offset;
        if let Some(&Friction(coeff)) = friction {
            if collision {
                let friction_coeff = (coeff + block_sum_frictions) / (frictions + 1) as f32;
                //rejection is component of velocity perpendicular to normal force
                let perp = v.reject_from(normal);
                let perp_length = perp.length();
                let dir = if perp_length.is_finite() && perp_length != 0.0 {
                    perp / perp_length
                } else {
                    Vec3::ZERO
                };
                //friction cannot be more than the current acceleration
                let magnitude = (friction_coeff * normal.length()).min(perp_length);
                *a -= magnitude * dir;
            }
        }
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
                    extents: Vec3::new(aabb.extents.x, 0.0, aabb.extents.z)
                        * Self::FACE_SHRINK_MULT,
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
                    extents: Vec3::new(aabb.extents.x, 0.0, aabb.extents.z)
                        * Self::FACE_SHRINK_MULT,
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
                    extents: Vec3::new(0.0, aabb.extents.y, aabb.extents.z)
                        * Self::FACE_SHRINK_MULT,
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
                    extents: Vec3::new(0.0, aabb.extents.y, aabb.extents.z)
                        * Self::FACE_SHRINK_MULT,
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
                    extents: Vec3::new(aabb.extents.x, aabb.extents.y, 0.0)
                        * Self::FACE_SHRINK_MULT,
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
                    extents: Vec3::new(aabb.extents.x, aabb.extents.y, 0.0)
                        * Self::FACE_SHRINK_MULT,
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
    mut objects: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut Acceleration,
            &mut CollidingDirections,
            &Collider,
            Option<&Friction>,
        ),
        Without<IgnoreTerrainCollision>,
    >,
    block_physics: Query<(&BlockPhysics, Option<&Friction>)>,
    level: Res<Level>,
) {
    for (mut tf, mut v, mut a, mut directions, col, fric) in objects.iter_mut() {
        let mut corrected_position = tf.translation;
        let mut corrected_velocity = v.0;

        directions.0.set(
            DirectionFlags::PosY,
            col.resolve(
                col.get_potential_overlap(
                    corrected_position,
                    ColliderShape::potential_overlapping_blocks_pos_y,
                ),
                &mut corrected_position,
                &mut corrected_velocity,
                &mut a,
                Vec3::NEG_Y,
                None,
                &level,
                &block_physics,
                ColliderShape::resolve_terrain_collision_pos_y,
            ),
        );

        directions.0.set(
            DirectionFlags::NegY,
            col.resolve(
                col.get_potential_overlap(
                    corrected_position,
                    ColliderShape::potential_overlapping_blocks_neg_y,
                ),
                &mut corrected_position,
                &mut corrected_velocity,
                &mut a,
                Vec3::Y,
                fric,
                &level,
                &block_physics,
                ColliderShape::resolve_terrain_collision_neg_y,
            ),
        );
        //todo - resolve collisions in order of axis magnitudes
        //correct is using time to collision - but i don't care enough for now
        //this is causing the snapping / hitching / teleporting when you move into a wall
        //https://nightblade9.github.io/godot-gamedev/2020/a-primer-on-aabb-collision-resolution.html

        if corrected_velocity.x.abs() > corrected_velocity.z.abs() {
            //x velocity is larger, do x collisions before z
            directions.0.set(
                DirectionFlags::PosX,
                col.resolve(
                    col.get_potential_overlap(
                        corrected_position,
                        ColliderShape::potential_overlapping_blocks_pos_x,
                    ),
                    &mut corrected_position,
                    &mut corrected_velocity,
                    &mut a,
                    Vec3::NEG_X,
                    None,
                    &level,
                    &block_physics,
                    ColliderShape::resolve_terrain_collision_pos_x,
                ),
            );

            directions.0.set(
                DirectionFlags::NegX,
                col.resolve(
                    col.get_potential_overlap(
                        corrected_position,
                        ColliderShape::potential_overlapping_blocks_neg_x,
                    ),
                    &mut corrected_position,
                    &mut corrected_velocity,
                    &mut a,
                    Vec3::X,
                    None,
                    &level,
                    &block_physics,
                    ColliderShape::resolve_terrain_collision_neg_x,
                ),
            );

            directions.0.set(
                DirectionFlags::PosZ,
                col.resolve(
                    col.get_potential_overlap(
                        corrected_position,
                        ColliderShape::potential_overlapping_blocks_pos_z,
                    ),
                    &mut corrected_position,
                    &mut corrected_velocity,
                    &mut a,
                    Vec3::NEG_Z,
                    None,
                    &level,
                    &block_physics,
                    ColliderShape::resolve_terrain_collision_pos_z,
                ),
            );

            directions.0.set(
                DirectionFlags::NegZ,
                col.resolve(
                    col.get_potential_overlap(
                        corrected_position,
                        ColliderShape::potential_overlapping_blocks_neg_z,
                    ),
                    &mut corrected_position,
                    &mut corrected_velocity,
                    &mut a,
                    Vec3::Z,
                    None,
                    &level,
                    &block_physics,
                    ColliderShape::resolve_terrain_collision_neg_z,
                ),
            );
        } else {
            //z velocity is larger, do z collisions before x
            directions.0.set(
                DirectionFlags::PosZ,
                col.resolve(
                    col.get_potential_overlap(
                        corrected_position,
                        ColliderShape::potential_overlapping_blocks_pos_z,
                    ),
                    &mut corrected_position,
                    &mut corrected_velocity,
                    &mut a,
                    Vec3::NEG_Z,
                    None,
                    &level,
                    &block_physics,
                    ColliderShape::resolve_terrain_collision_pos_z,
                ),
            );

            directions.0.set(
                DirectionFlags::NegZ,
                col.resolve(
                    col.get_potential_overlap(
                        corrected_position,
                        ColliderShape::potential_overlapping_blocks_neg_z,
                    ),
                    &mut corrected_position,
                    &mut corrected_velocity,
                    &mut a,
                    Vec3::Z,
                    None,
                    &level,
                    &block_physics,
                    ColliderShape::resolve_terrain_collision_neg_z,
                ),
            );

            directions.0.set(
                DirectionFlags::PosX,
                col.resolve(
                    col.get_potential_overlap(
                        corrected_position,
                        ColliderShape::potential_overlapping_blocks_pos_x,
                    ),
                    &mut corrected_position,
                    &mut corrected_velocity,
                    &mut a,
                    Vec3::NEG_X,
                    None,
                    &level,
                    &block_physics,
                    ColliderShape::resolve_terrain_collision_pos_x,
                ),
            );

            directions.0.set(
                DirectionFlags::NegX,
                col.resolve(
                    col.get_potential_overlap(
                        corrected_position,
                        ColliderShape::potential_overlapping_blocks_neg_x,
                    ),
                    &mut corrected_position,
                    &mut corrected_velocity,
                    &mut a,
                    Vec3::X,
                    None,
                    &level,
                    &block_physics,
                    ColliderShape::resolve_terrain_collision_neg_x,
                ),
            );
        }

        tf.translation = corrected_position;
        v.0 = corrected_velocity;
    }
}
