use bevy::prelude::*;

use crate::{
    ui::{debug::DebugBlockHitboxes, state::DebugUIState},
    util::{
        iterators::{AxisIter, BlockVolume, AxisMap},
        DirectionFlags, project_onto_plane,
    },
    world::{BlockCoord, BlockPhysics, Level},
};

use super::{movement::*, PhysicsSystemSet, TICK_SCALE};

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

    pub fn with_extents(self, extents: Vec3) -> Self {
        Collider {
            shape: Aabb::new(extents),
            offset: self.offset,
        }
    }

    //returns the block coordinate we collided with, corrected velocity, time, normal if exists
    pub fn min_time_to_collision<'a>(
        &self,
        potential_overlap: impl Iterator<Item = (BlockCoord, &'a BlockPhysics)>,
        p: Vec3,
        v: Vec3,
    ) -> Option<(BlockCoord, Vec3, f32, Option<crate::util::Direction>)> {
        let position = p + self.offset;
        let mut min_collision: Option<(BlockCoord, Vec3, f32, Option<crate::util::Direction>)> =
            None;
        for (coord, block) in potential_overlap {
            if let Some(block_collider) = Collider::from_block(block) {
                if let Some((time, corrected_v, opt_normal)) = self.shape.sweep(
                    coord.to_vec3() + block_collider.offset - position,
                    block_collider.shape,
                    v,
                ) {
                    match min_collision {
                        Some((_, _, min_collision_time, _)) => {
                            if time < min_collision_time {
                                min_collision = Some((coord, corrected_v, time, opt_normal));
                            }
                        }
                        None => {
                            min_collision = Some((coord, corrected_v, time, opt_normal));
                        }
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

    pub fn intersects_point(&self, delta: Vec3) -> bool {
        self.shape.intersects_point(delta - self.offset)
    }

    const FACE_SIZE_MULT: f32 = 31. / 32.; //shrinks each face on the box collider by this proportion to avoid conflicting collisions against walls

    pub fn potential_overlapping_blocks_pos_y(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(self.shape.extents.x, 0.0, self.shape.extents.z) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta + Vec3::new(0., self.shape.extents.y, 0.),
        )
    }

    pub fn potential_overlapping_blocks_neg_y(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb {
                extents: Vec3::new(self.shape.extents.x, 0.0, self.shape.extents.z)
                    * Self::FACE_SIZE_MULT,
            },
            self.offset + delta - Vec3::new(0., self.shape.extents.y, 0.),
        )
    }

    pub fn potential_overlapping_blocks_pos_x(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(0.0, self.shape.extents.y, self.shape.extents.z) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta + Vec3::new(self.shape.extents.x, 0., 0.),
        )
    }

    pub fn potential_overlapping_blocks_neg_x(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(0.0, self.shape.extents.y, self.shape.extents.z) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta - Vec3::new(self.shape.extents.x, 0., 0.),
        )
    }

    pub fn potential_overlapping_blocks_pos_z(&self, delta: Vec3) -> BlockVolume {
        BlockVolume::from_aabb(
            Aabb::new(
                Vec3::new(self.shape.extents.x, self.shape.extents.y, 0.0) * Self::FACE_SIZE_MULT,
            ),
            self.offset + delta + Vec3::new(0., 0., self.shape.extents.z),
        )
    }

    pub fn potential_overlapping_blocks_neg_z(&self, delta: Vec3) -> BlockVolume {
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
#[derive(Clone, Copy, Debug, Reflect, PartialEq)]
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

        (self_min.x <= other_max.x && self_max.x >= other_min.x)
            && (self_min.y <= other_max.y && self_max.y >= other_min.y)
            && (self_min.z <= other_max.z && self_max.z >= other_min.z)
    }

    pub fn intersects_point(self, other: Vec3) -> bool {
        other
            .axis_iter()
            .zip(self.extents.axis_iter())
            .all(|(other, extent)| other <= extent)
    }

    //self.position + delta = other.position
    //returns the dispacement that we have to move self on each axis to hit other box (0 if inside)
    //returns infinity if there is no possible collision on that axis
    pub fn axis_displacement(self, other_center: Vec3, other: Aabb) -> Vec3 {
        let self_min = -self.extents;
        let self_max = self.extents;
        let other_min = other_center - other.extents;
        let other_max = other_center + other.extents;
        let mut dist = Vec3::splat(f32::INFINITY);
        //x-axis, check if possible boxes are lined up enough on y/z plane for collision
        if (self_min.y <= other_max.y && self_max.y >= other_min.y)
            && (self_min.z <= other_max.z && self_max.z >= other_min.z)
        {
            let d = ((other_min.x - self_max.x).abs()).min((self_min.x - other_max.x).abs());
            dist.x = if self_min.x <= other_max.x && self_max.x >= other_min.x {
                0.0 //intersects
            } else {
                d * other_center.x.signum()
            }
        }
        //y-axis, check if possible boxes are lined up enough on x/z plane for collision
        if (self_min.x <= other_max.x && self_max.x >= other_min.x)
            && (self_min.z <= other_max.z && self_max.z >= other_min.z)
        {
            let d = ((other_min.y - self_max.y).abs()).min((self_min.y - other_max.y).abs());
            dist.y = if self_min.y <= other_max.y && self_max.y >= other_min.y {
                0.0 //intersects
            } else {
                d * other_center.y.signum()
            }
        }
        //z-axis, check if possible boxes are lined up enough on x/y plane for collision
        if (self_min.y <= other_max.y && self_max.y >= other_min.y)
            && (self_min.x <= other_max.x && self_max.x >= other_min.x)
        {
            let d = ((other_min.z - self_max.z).abs()).min((self_min.z - other_max.z).abs());
            dist.z = if self_min.z <= other_max.z && self_max.z >= other_min.z {
                0.0
            } else {
                d * other_center.z.signum()
            }
        }
        dist
    }

    //returns time to hit, displacement until hit, and normal if there was a hit
    //returns None if no hit
    pub fn sweep(
        self,
        other_center: Vec3,
        other: Aabb,
        v: Vec3,
    ) -> Option<(f32, Vec3, Option<crate::util::Direction>)> {
        if self.intersects(other_center, other) {
            return Some((0.0, Vec3::ZERO, None));
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
        } else {
            entry_dist.x = other_max.x - self_min.x;
            exit_dist.x = other_min.x - self_max.x;
        }

        if v.y > 0.0 {
            entry_dist.y = other_min.y - self_max.y;
            exit_dist.y = other_max.y - self_min.y;
        } else {
            entry_dist.y = other_max.y - self_min.y;
            exit_dist.y = other_min.y - self_max.y;
        }

        if v.z > 0.0 {
            entry_dist.z = other_min.z - self_max.z;
            exit_dist.z = other_max.z - self_min.z;
        } else {
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
            Some(if v[max_entry_index] < 0.0 {
                normal_axis
            } else {
                normal_axis.opposite()
            }),
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
        let mut effective_velocity = TICK_SCALE as f32 * (v.0 + TICK_SCALE as f32 * 0.5 * a.0);
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
            if let Some((block_pos, corrected_v, time, opt_normal)) =
                col.min_time_to_collision(overlaps_iter.clone(), tf.translation, effective_velocity)
            {
                if *debug_state == DebugUIState::Shown {
                    block_gizmos.hit_blocks.insert(block_pos);
                    info!("tf: {:?}, time_remainig: {:?}\n", tf.translation, time);
                }
                tf.translation += corrected_v;
                time_remaining -= time;
                
                //do velocity resolution and set collision direction flag
                //direction flag is opposite direction because the direction is the normal of the collision, and directionflag is
                //  the direction relative to the entity.
                // info!("hit normal: {:?}", opt_normal);
                match opt_normal {
                    Some(dir) => {
                        directions.0.set(dir.opposite().into(), true);
                        //slide perpendicular to the collision normal
                        // let normal = dir.to_vec3();
                        // let speed = effective_velocity.dot(normal) * time_remaining;
                        // effective_velocity = project_onto_plane(effective_velocity, normal)*speed;
                        // v.0 = effective_velocity;
                        let idx = dir.to_idx() % 3;
                        effective_velocity[idx] = 0.0;
                        v.0 = effective_velocity;
                    },
                    None => {
                        //we are inside a block already
                        warn!("inside block!");
                        v.0 = Vec3::ZERO;
                        directions.0.set(DirectionFlags::all(), true);
                    }
                }
            } else {
                //no collision, so no need to do other iterations
                tf.translation += effective_velocity;
                break;
            }
        }
        // info!("directions: {:?}", directions.0);
    }
}
