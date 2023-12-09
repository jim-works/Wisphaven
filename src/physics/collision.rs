use bevy::prelude::*;

use crate::{
    util::{
        iterators::{AxisIter, BlockVolume, VolumeContainer},
        DirectionFlags,
    },
    world::{BlockCoord, BlockPhysics, Level},
};

use super::{movement::*, PhysicsSystemSet};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            resolve_terrain_collisions.in_set(PhysicsSystemSet::CollisionResolution),
        )
        .register_type::<Collider>()
        .register_type::<Aabb>();
    }
}

#[derive(Component)]
pub struct Friction(f32);

impl Default for Friction {
    fn default() -> Self {
        Self(0.005)
    }
}

#[derive(Component, Copy, Clone, PartialEq, Default, Reflect, Debug)]
#[reflect(Component)]
pub struct Collider {
    pub shape: Aabb,
    pub offset: Vec3,
}

#[derive(Component, Default)]
pub struct CollidingDirections(pub DirectionFlags);

impl Collider {
    pub fn from_block(physics: &BlockPhysics) -> Option<Self> {
        match physics {
            BlockPhysics::Empty => None,
            BlockPhysics::Solid => Some(Collider {
                shape: Aabb::new(Vec3::splat(0.5)),
                offset: Vec3::splat(0.5),
            }),
            BlockPhysics::Aabb(col) => Some(*col),
        }
    }

    pub fn min_time_to_collision<'a>(
        &self,
        potential_overlap: impl Iterator<Item = (BlockCoord, &'a BlockPhysics)>,
        p: Vec3,
        v: Vec3,
    ) -> Option<(BlockCoord, Vec3, f32)> {
        let position = p + self.offset;
        let mut min_collision = None;
        for (coord, block) in potential_overlap {
            if let Some(block_collider) = Collider::from_block(block) {
                let d = self.shape.axis_distance(
                    coord.to_vec3() + block_collider.offset - position,
                    block_collider.shape,
                );
                let t = Vec3::new(
                    if v.x == 0.0 { f32::INFINITY } else { d.x / v.x },
                    if v.y == 0.0 { f32::INFINITY } else { d.y / v.y },
                    if v.z == 0.0 { f32::INFINITY } else { d.z / v.z },
                );
                let min = f32::min(t.x, f32::min(t.y, t.z));
                if min == f32::INFINITY {
                    continue;
                }
                match min_collision {
                    Some((_, _, min_t)) => {
                        if min < min_t {
                            min_collision = Some((coord, t, min));
                        }
                    }
                    None => {
                        min_collision = Some((coord, t, min));
                    }
                }
            }
        }
        min_collision
    }

    fn calc_friction() {
        // let mut block_sum_frictions = 0.;
        // let mut frictions = 0;
        // if let Some(&Friction(coeff)) = friction {
        //     if collision {
        //         let friction_coeff = (coeff + block_sum_frictions) / (frictions + 1) as f32;
        //         //rejection is component of velocity perpendicular to normal force
        //         let perp = v.reject_from(normal);
        //         let perp_length = perp.length();
        //         let dir = if perp_length.is_finite() && perp_length != 0.0 {
        //             perp / perp_length
        //         } else {
        //             Vec3::ZERO
        //         };
        //         //friction cannot be more than the current acceleration
        //         let magnitude = (friction_coeff * normal.length()).min(perp_length);
        //         *a -= magnitude * dir;
        //     }
        // }
    }

    const FACE_SHRINK_MULT: f32 = 0.99; //shrinks each face on the box collider by this proportion to avoid conflicting collisions against walls

    // fn resolve(
    //     &self,
    //     p: &mut Vec3,
    //     v: &mut Vec3,
    //     level: &Level,
    //     block_physics: &Query<&BlockPhysics>,
    //     potential_overlap: BlockVolume,
    //     mut resolver: impl FnMut(
    //         &ColliderShape,
    //         BlockCoord,
    //         Option<&BlockPhysics>,
    //         &mut Vec3,
    //         &mut Vec3,
    //     ) -> bool,
    // ) -> bool {
    //     let mut collision = false;
    //     for coord in potential_overlap.iter() {
    //         collision |= resolver(
    //             self,
    //             coord,
    //             level
    //                 .get_block_entity(coord)
    //                 .and_then(|b| block_physics.get(b).ok()),
    //             p,
    //             v,
    //         );
    //     }
    //     collision
    // }

    pub fn potential_overlapping_blocks_neg_y(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb {
                extents: Vec3::new(self.shape.extents.x, 0.0, self.shape.extents.z)
                    * Self::FACE_SHRINK_MULT,
            },
            self.offset + delta - Vec3::new(0., self.shape.extents.y, 0.),
        )
    }

    // //assumes block is in the set returned by potential overlapping blocks
    // fn resolve_terrain_collision_neg_y(
    //     &self,
    //     coord: BlockCoord,
    //     block_physics: Option<&BlockPhysics>,
    //     position: &mut Vec3,
    //     velocity: &mut Vec3,
    // ) -> bool {
    //     match self {
    //         ColliderShape::Box(aabb) => match block_physics {
    //             Some(BlockPhysics::Solid) => {
    //                 velocity.y = 0.0;
    //                 position.y = (coord.y + 1) as f32 + aabb.extents.y;
    //                 true
    //             }
    //             Some(BlockPhysics::BottomSlab(_height)) => {
    //                 todo!()
    //             }
    //             Some(BlockPhysics::Empty) | None => false,
    //         },
    //     }
    // }

    pub fn potential_overlapping_blocks_pos_y(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(self.shape.extents.x, 0.0, self.shape.extents.z) * Self::FACE_SHRINK_MULT,
            ),
            self.offset + delta + Vec3::new(0., self.shape.extents.y, 0.),
        )
    }

    pub fn potential_overlapping_blocks_pos_x(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(0.0, self.shape.extents.y, self.shape.extents.z) * Self::FACE_SHRINK_MULT,
            ),
            self.offset + delta + Vec3::new(self.shape.extents.x, 0., 0.),
        )
    }

    pub fn potential_overlapping_blocks_neg_x(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(0.0, self.shape.extents.y, self.shape.extents.z) * Self::FACE_SHRINK_MULT,
            ),
            self.offset + delta - Vec3::new(self.shape.extents.x, 0., 0.),
        )
    }

    pub fn potential_overlapping_blocks_pos_z(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(self.shape.extents.x, self.shape.extents.y, 0.0) * Self::FACE_SHRINK_MULT,
            ),
            self.offset + delta + Vec3::new(0., 0., self.shape.extents.z),
        )
    }

    pub fn potential_overlapping_blocks_neg_z(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(self.shape.extents.x, self.shape.extents.y, 0.0) * Self::FACE_SHRINK_MULT,
            ),
            self.offset + delta - Vec3::new(0., 0., self.shape.extents.z),
        )
    }
}

//axis-aligned bounding box
//not a collider atm
#[derive(Clone, Copy, Debug, Reflect, PartialEq)]
pub struct Aabb {
    pub extents: Vec3,
}

impl Aabb {
    pub fn new(extents: Vec3) -> Self {
        Self { extents }
    }
    //self.position + delta = other.position
    pub fn intersects(self, delta: Vec3, other: Aabb) -> bool {
        self.axis_distance(delta, other).axis_iter().all(|d| d <= 0.0)
    }

    //self.position + delta = other.position
    //returns distance from outside of this box to other box (negative distance if inside)
    //todo: this should return the distance that we have to move in each axis to hit the box (so infinity if no collision)
    pub fn axis_distance(self, delta: Vec3, other: Aabb) -> Vec3 {
        delta - self.extents - other.extents
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self {
            extents: Vec3::new(0.5, 0.5, 0.5),
        }
    }
}

#[derive(Component, Copy, Clone)]
pub struct IgnoreTerrainCollision;

#[derive(Component, Copy, Clone, Default, Debug)]
pub struct DesiredPosition(pub Vec3);

fn resolve_terrain_collisions(
    mut objects: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut Acceleration,
            &mut CollidingDirections,
            &Collider,
            &mut DesiredPosition,
            Option<&Friction>,
        ),
        Without<IgnoreTerrainCollision>,
    >,
    block_physics: Query<&BlockPhysics>,
    level: Res<Level>,
) {
    let mut overlaps: Vec<VolumeContainer<crate::world::BlockType>> = Vec::with_capacity(3);
    for (mut tf, mut v, mut a, mut directions, col, mut desired_pos, fric) in objects.iter_mut() {
        directions.0 = DirectionFlags::default();
        // info!("tf: {:?}, v: {:?}, desired: {:?}", tf, v, desired_pos);
        if v.x > 0.0 {
            overlaps.push(
                level.get_blocks_in_volume(col.potential_overlapping_blocks_pos_x(desired_pos.0)),
            );
        } else if v.x < 0.0 {
            overlaps.push(
                level.get_blocks_in_volume(col.potential_overlapping_blocks_neg_x(desired_pos.0)),
            );
        }

        if v.y > 0.0 {
            overlaps.push(
                level.get_blocks_in_volume(col.potential_overlapping_blocks_pos_y(desired_pos.0)),
            );
        } else if v.y < 0.0 {
            overlaps.push(
                level.get_blocks_in_volume(col.potential_overlapping_blocks_neg_y(desired_pos.0)),
            );
        }

        if v.z > 0.0 {
            overlaps.push(
                level.get_blocks_in_volume(col.potential_overlapping_blocks_pos_z(desired_pos.0)),
            );
        } else if v.z < 0.0 {
            overlaps.push(
                level.get_blocks_in_volume(col.potential_overlapping_blocks_neg_z(desired_pos.0)),
            );
        }
        let overlaps_iter = overlaps.iter().flat_map(|v| {
            v.iter().filter_map(|(coord, block)| {
                block
                    .and_then(|b| b.entity())
                    .and_then(|e| block_physics.get(e).ok())
                    .and_then(|p| Some((coord, p)))
            })
        });
        let min_time_to_collision = col.min_time_to_collision(overlaps_iter, tf.translation, v.0);
        if let Some((_block_pos, times, min_time)) = min_time_to_collision {
            info!("min_time: {:?}", min_time);
            desired_pos.0 = tf.translation + min_time * v.0;
            //do velocity resolution and set collision direction flag
            if times.x == min_time {
                if v.0.x > 0.0 {
                    directions.0.set(DirectionFlags::PosX, true);
                }
                if v.0.x < 0.0 {
                    directions.0.set(DirectionFlags::NegX, true);
                }
                v.0.x = 0.0;
            }
            if times.y == min_time {
                if v.0.y > 0.0 {
                    directions.0.set(DirectionFlags::PosY, true);
                }
                if v.0.y < 0.0 {
                    directions.0.set(DirectionFlags::NegY, true);
                }
                v.0.y = 0.0;
            }
            if times.z == min_time {
                if v.0.z > 0.0 {
                    directions.0.set(DirectionFlags::PosZ, true);
                }
                if v.0.z < 0.0 {
                    directions.0.set(DirectionFlags::NegZ, true);
                }
                v.0.z = 0.0;
            }
        }

        tf.translation = desired_pos.0;
    }
}
