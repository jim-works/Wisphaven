use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    ui::{debug::DebugBlockHitboxes, state::DebugUIState},
    util::{
        iterators::{AxisIter, AxisMap, BlockVolume},
        project_onto_plane, DirectionFlags,
    },
    world::{BlockCoord, BlockPhysics, Level},
};

use super::{movement::*, PhysicsSystemSet};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            move_and_slide.in_set(PhysicsSystemSet::UpdatePosition),
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

#[derive(Component, Clone, Copy, PartialEq, Default, Reflect, Debug, Serialize, Deserialize)]
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

    pub fn with_extents(self, extents: Vec3) -> Self {
        Collider {
            shape: Aabb::new(extents),
            offset: self.offset,
        }
    }

    //returns the block coordinate we collided with, corrected velocity, time, normal if exists
    pub fn min_time_to_collision<'a>(
        self,
        potential_overlap: impl Iterator<Item = (BlockCoord, &'a BlockPhysics)>,
        p: Vec3,
        v: Vec3,
    ) -> Option<(BlockCoord, Vec3, f32, crate::util::Direction)> {
        let position = p + self.offset;
        let mut min_collision: Option<(BlockCoord, Vec3, f32, crate::util::Direction)> = None;
        for (coord, block) in potential_overlap {
            if let Some(block_collider) = Collider::from_block(block) {
                if let Some((time, corrected_v, normal)) = self.shape.sweep(
                    coord.to_vec3() + block_collider.offset - position,
                    block_collider.shape,
                    v,
                ) {
                    match min_collision {
                        Some((_, _, min_collision_time, _)) => {
                            if time < min_collision_time {
                                min_collision = Some((coord, corrected_v, time, normal));
                            }
                        }
                        None => {
                            min_collision = Some((coord, corrected_v, time, normal));
                        }
                    }
                }
            }
        }
        info!("min collision {:?}", min_collision);
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

    pub fn intersects_point(self, delta: Vec3) -> bool {
        self.shape.intersects_point(delta - self.offset)
    }

    const FACE_SIZE_MULT: f32 = 31. / 32.; //shrinks each face on the box collider by this proportion to avoid conflicting collisions against walls

    pub fn potential_overlapping_blocks_pos_y(self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(self.shape.extents.x, 0.0, self.shape.extents.z) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta + Vec3::new(0., self.shape.extents.y, 0.),
        )
    }

    pub fn potential_overlapping_blocks_neg_y(self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb {
                extents: Vec3::new(self.shape.extents.x, 0.0, self.shape.extents.z)
                    * Self::FACE_SIZE_MULT,
            },
            self.offset + delta - Vec3::new(0., self.shape.extents.y, 0.),
        )
    }

    pub fn potential_overlapping_blocks_pos_x(self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(0.0, self.shape.extents.y, self.shape.extents.z) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta + Vec3::new(self.shape.extents.x, 0., 0.),
        )
    }

    pub fn potential_overlapping_blocks_neg_x(self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(0.0, self.shape.extents.y, self.shape.extents.z) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta - Vec3::new(self.shape.extents.x, 0., 0.),
        )
    }

    pub fn potential_overlapping_blocks_pos_z(self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(self.shape.extents.x, self.shape.extents.y, 0.0) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta + Vec3::new(0., 0., self.shape.extents.z),
        )
    }

    pub fn potential_overlapping_blocks_neg_z(self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(self.shape.extents.x, self.shape.extents.y, 0.0) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta - Vec3::new(0., 0., self.shape.extents.z),
        )
    }
}

//axis-aligned bounding box
//not a collider atm
#[derive(Component, Copy, Clone, PartialEq, Reflect, Debug, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Aabb {
    pub extents: Vec3,
}

impl Aabb {
    pub fn new(extents: Vec3) -> Self {
        Self { extents }
    }
    pub fn intersects(self, other_center: Vec3, other: Aabb) -> bool {
        let self_min = -self.extents;
        let self_max = self.extents;
        let other_min = other_center - other.extents;
        let other_max = other_center + other.extents;

        (self_min.x < other_max.x && self_max.x > other_min.x)
            && (self_min.y < other_max.y && self_max.y > other_min.y)
            && (self_min.z < other_max.z && self_max.z > other_min.z)
    }

    //how much does self penetrate other?
    //self.pos + other_center = other.pos
    pub fn penetration_vector(self, delta: Vec3, other: Aabb) -> Vec3 {
        (self.extents, delta, other.extents).axis_map(
            |(self_extent, other_center, other_extent)| {
                if other_center > 0. {
                    //self_max - other-min
                    self_extent - (other_center - other_extent)
                } else {
                    //self_min - other_max
                    -self_extent - (other_center + other_extent)
                }
            },
        )
    }

    pub fn intersects_point(self, other: Vec3) -> bool {
        other
            .axis_iter()
            .zip(self.extents.axis_iter())
            .all(|(other, extent)| other <= extent)
    }

    //self.position + delta = other.position
    //returns the dispacement that we have to move self on each axis to move outside of other box
    //I have no idea what this returns if the boxes aren't overlapping
    pub fn overlapping_displacement(self, other_center: Vec3, other: Aabb) -> Vec3 {
        let self_min = -self.extents;
        let self_max = self.extents;
        let other_min = other_center - other.extents;
        let other_max = other_center + other.extents;

        //if there is a positive displacement here, the shortest way to reslove the collision is to add
        let positive_displacement = (other_max - self_min).abs();
        //if there is a positive displacement here, the shortest way to resolve the collision is to subtract
        let negative_displacement = (other_min - self_max).abs();
        //since these are aabbs, if they overlap, the smallest magntiude would be the shortest way out.

        (positive_displacement, negative_displacement)
            .axis_map(|(p, n)| if p <= n { p } else { -n })
    }

    //returns time to hit, displacement until hit, and normal if there was a hit
    //returns None if no hit
    pub fn sweep(
        self,
        other_center: Vec3,
        other: Aabb,
        v: Vec3,
    ) -> Option<(f32, Vec3, crate::util::Direction)> {
        if self.intersects(other_center, other) {
            //already intersecting, return shortest vector to correct intersection
            let correction_candidates = self.overlapping_displacement(other_center, other);
            let min_correction = crate::util::min_index(correction_candidates);
            let picked_correction = crate::util::pick_axis(correction_candidates, min_correction);
            println!(
                "balls {:?} ex {:?}, self ex {:?} correction {:?}",
                other_center, other, self, picked_correction
            );
            return Some((
                0.0,
                -picked_correction,
                if picked_correction[min_correction] <= 0. {
                    crate::util::Direction::from(min_correction).opposite()
                } else {
                    crate::util::Direction::from(min_correction)
                },
            ));
        }
        if v == Vec3::ZERO {
            return None;
        }

        let self_min = -self.extents;
        let self_max = self.extents;
        let other_min = other_center - other.extents;
        let other_max = other_center + other.extents;

        let mut entry_dist = Vec3::ZERO;
        let mut exit_dist = Vec3::ZERO;
        let mut entry_time = Vec3::ZERO;
        let mut exit_time = Vec3::ZERO;

        //find the distances between the near and far side of the other box for each axis
        if v.x > 0.0 {
            entry_dist.x = other_min.x - self_max.x;
            exit_dist.x = other_max.x - self_min.x;
        } else if v.x < 0.0 {
            entry_dist.x = other_max.x - self_min.x;
            exit_dist.x = other_min.x - self_max.x;
        }

        if v.y > 0.0 {
            entry_dist.y = other_min.y - self_max.y;
            exit_dist.y = other_max.y - self_min.y;
        } else if v.y < 0.0 {
            entry_dist.y = other_max.y - self_min.y;
            exit_dist.y = other_min.y - self_max.y;
        }

        if v.z > 0.0 {
            entry_dist.z = other_min.z - self_max.z;
            exit_dist.z = other_max.z - self_min.z;
        } else if v.z < 0.0 {
            entry_dist.z = other_max.z - self_min.z;
            exit_dist.z = other_min.z - self_max.z;
        }

        //find the time of entry and time of exit for each axis
        if v.x == 0.0 {
            entry_time.x = f32::NEG_INFINITY;
            exit_time.x = f32::INFINITY;
        } else {
            entry_time.x = entry_dist.x / v.x;
            exit_time.x = exit_dist.x / v.x;
        }

        if v.y == 0.0 {
            entry_time.y = f32::NEG_INFINITY;
            exit_time.y = f32::INFINITY;
        } else {
            entry_time.y = entry_dist.y / v.y;
            exit_time.y = exit_dist.y / v.y;
        }

        if v.z == 0.0 {
            entry_time.z = f32::NEG_INFINITY;
            exit_time.z = f32::INFINITY;
        } else {
            entry_time.z = entry_dist.z / v.z;
            exit_time.z = exit_dist.z / v.z;
        }

        entry_time = entry_time.axis_map(|x| if x > 1.0 { f32::NEG_INFINITY } else { x });
        let max_entry_index = crate::util::max_index(entry_time);
        let max_entry_time = entry_time[max_entry_index];
        let min_exit_time = exit_time.min_element();

        if max_entry_time > min_exit_time {
            //we already left the collision before we intersected on one axis
            return None;
        }
        if entry_time.x < 0.0 && entry_time.y < 0.0 && entry_time.z < 0.0 {
            return None;
        }
        if entry_time.x < 0.0 && (self_max.x < other_min.x || other_max.x < self_min.x) {
            return None;
        }
        if entry_time.y < 0.0 && (self_max.y < other_min.y || other_max.y < self_min.y) {
            return None;
        }
        if entry_time.z < 0.0 && (self_max.z < other_min.z || other_max.z < self_min.z) {
            return None;
        }

        //want axis with minimal collision time
        //pos/neg direction on axis is determined by relative velocity
        let normal_axis = crate::util::Direction::from(max_entry_index);
        return Some((
            max_entry_time,
            v * max_entry_time,
            if v[max_entry_index] < 0.0 {
                normal_axis
            } else {
                normal_axis.opposite()
            },
        ));
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self {
            extents: Vec3::new(0.5, 0.5, 0.5),
        }
    }
}

#[derive(Component, Copy, Clone, Default)]
pub struct ProcessTerrainCollision;

fn move_and_slide(
    mut objects: Query<
        (
            &mut Transform,
            &mut Velocity,
            &Acceleration,
            &mut CollidingDirections,
            &Collider,
        ),
        With<ProcessTerrainCollision>,
    >,
    block_physics: Query<&BlockPhysics>,
    mut block_gizmos: ResMut<DebugBlockHitboxes>,
    level: Res<Level>,
    debug_state: Res<State<DebugUIState>>,
) {
    block_gizmos.blocks.clear();
    block_gizmos.hit_blocks.clear();
    // let mut overlaps: Vec<VolumeContainer<crate::world::BlockType>> = Vec::with_capacity(3);
    for (mut tf, mut v, a, mut directions, col) in objects.iter_mut() {
        // overlaps.clear();
        directions.0 = DirectionFlags::default();
        let mut effective_velocity = v.0;
        v.0 += a.0;
        let mut time_remaining = 1.0;

        //collide on one axis at a time, repeat 3 times in case we are colliding on all 3 axes
        for _ in 0..3 {
            //todo - optimize updates
            // overlaps.clear();
            // if v_remaining.x > 0.0 {
            //     overlaps.push(level.get_blocks_in_volume(
            //         col.potential_overlapping_blocks_pos_x(tf.translation + v_remaining),
            //     ));
            // } else if v_remaining.x < 0.0 {
            //     overlaps.push(level.get_blocks_in_volume(
            //         col.potential_overlapping_blocks_neg_x(tf.translation + v_remaining),
            //     ));
            // }

            // if v_remaining.y > 0.0 {
            //     overlaps.push(level.get_blocks_in_volume(
            //         col.potential_overlapping_blocks_pos_y(tf.translation + v_remaining),
            //     ));
            // } else if v_remaining.y < 0.0 {
            //     overlaps.push(level.get_blocks_in_volume(
            //         col.potential_overlapping_blocks_neg_y(tf.translation + v_remaining),
            //     ));
            // }

            // if v_remaining.z > 0.0 {
            //     overlaps.push(level.get_blocks_in_volume(
            //         col.potential_overlapping_blocks_pos_z(tf.translation + v_remaining),
            //     ));
            // } else if v_remaining.z < 0.0 {
            //     overlaps.push(level.get_blocks_in_volume(
            //         col.potential_overlapping_blocks_neg_z(tf.translation + v_remaining),
            //     ));
            // }
            // let overlaps_iter = overlaps.iter().flat_map(|v| {
            //     v.iter().filter_map(|(coord, block)| {
            //         block
            //             .and_then(|b| b.entity())
            //             .and_then(|e| block_physics.get(e).ok())
            //             .and_then(|p| Some((coord, p)))
            //     })
            // });
            let overlaps = level.get_blocks_in_volume(BlockVolume::new_inclusive(
                BlockCoord::from(tf.translation - col.shape.extents * 10.0),
                BlockCoord::from(tf.translation + col.shape.extents * 10.0),
            ));
            let overlaps_iter = overlaps.iter().filter_map(|(coord, block)| {
                block
                    .and_then(|b| b.entity())
                    .and_then(|e| block_physics.get(e).ok())
                    .and_then(|p| Some((coord, p)))
            });
            if *debug_state == DebugUIState::Shown {
                let gizmos_iter = overlaps.iter().map(|(coord, block)| {
                    (
                        coord,
                        block
                            .and_then(|b| b.entity())
                            .and_then(|e| block_physics.get(e).ok())
                            .and_then(|p| Some(p.clone())),
                    )
                });
                block_gizmos.blocks.extend(gizmos_iter);
            }
            // info!("\neffective_velocity: {:?}", effective_velocity);
            // info!("tf: {:?}, v_remaining: {:?}\n", tf.translation, v_remaining);
            // for (coord, block) in overlaps_iter {
            //     if let Some(block_collider) = Collider::from_block(block) {
            //         let block_offset = coord.to_vec3() + block_collider.offset - tf.translation;
            //         if col.shape.intersects(block_offset, block_collider.shape) {
            //             block_gizmos.hit_blocks.insert(coord);
            //             let correction_candidates = col
            //                 .shape
            //                 .overlapping_displacement(block_offset, block_collider.shape);
            //             let min_correction = crate::util::min_index(correction_candidates);
            //             let picked_correction =
            //                 crate::util::pick_axis(correction_candidates, min_correction);
            //             effective_velocity[min_correction] = 0.0;
            //             tf.translation += picked_correction;
            //             directions.0.set(
            //                 if picked_correction[min_correction] <= 0. {
            //                     crate::util::Direction::from(min_correction)
            //                 } else {
            //                     crate::util::Direction::from(min_correction).opposite()
            //                 }
            //                 .into(),
            //                 true,
            //             );
            //         }
            //     }
            // }

            // if let Some((block_pos, corrected_v, time, normal)) =
            //     col.min_time_to_collision(overlaps_iter.clone(), tf.translation, effective_velocity)
            // {
            //     if *debug_state == DebugUIState::Shown {
            //         block_gizmos.hit_blocks.insert(block_pos);
            //         info!(
            //             "tf: {:?}, time_remainig: {:?}, v: {:?}, v_corrected: {:?}\n",
            //             tf.translation, time, effective_velocity, corrected_v
            //         );
            //     }
            //     tf.translation += corrected_v;
            //     time_remaining -= time;

            //     //do velocity resolution and set collision direction flag
            //     //direction flag is opposite direction because the direction is the normal of the collision, and directionflag is
            //     //  the direction relative to the entity.
            //     // info!("hit normal: {:?}", opt_normal);
            //     directions.0.set(normal.opposite().into(), true);
            //     //slide perpendicular to the collision normal
            //     // let normal = dir.to_vec3();
            //     // let speed = effective_velocity.dot(normal) * time_remaining;
            //     // effective_velocity = project_onto_plane(effective_velocity, normal)*speed;
            //     // v.0 = effective_velocity;
            //     effective_velocity = corrected_v;
            //     v.0 = effective_velocity;
            // } else {
            //     //no collision, so no need to do other iterations
            //     tf.translation += effective_velocity;
            //     break;
            // }

            for (coord, block) in overlaps_iter.clone() {
                if let Some(block_collider) = Collider::from_block(block) {
                    let block_offset = coord.to_vec3() + block_collider.offset - tf.translation;
                    if !col.shape.intersects(block_offset, block_collider.shape) {
                        continue;
                    }
                    if *debug_state == DebugUIState::Shown {
                        block_gizmos.hit_blocks.insert(coord);
                    }
                    let penetration = col.shape.penetration_vector(block_offset, block_collider.shape);
                    let min_penetration_idx = crate::util::min_index(penetration);
                    let correction = -crate::util::pick_axis(penetration, min_penetration_idx);
                    //unsure if this if is necessary, can we just set the velocity in this axis to 0?
                    if v.0[min_penetration_idx].signum() * correction[min_penetration_idx] < 0.0 {
                        v.0[min_penetration_idx] = 0.0;
                        effective_velocity[min_penetration_idx] = 0.0;
                    }
                    //move out of the block
                    tf.translation += correction;
                    directions.0.set(DirectionFlags::all(), true);
                }
            }
        }
        tf.translation += effective_velocity;
        info!("directions: {:?}", directions.0);
    }
}
