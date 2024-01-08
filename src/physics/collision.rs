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
        .register_type::<Aabb>()
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
pub struct Aabb {
    pub size: Vec3,
    pub offset: Vec3,
}

#[derive(Component, Default)]
pub struct CollidingDirections(pub DirectionFlags);

impl Aabb {
    pub fn new(size: Vec3, offset: Vec3) -> Self {
        Self { size, offset }
    }
    //maintains center of mass, scale by factor
    pub fn scale(self, scale: Vec3) -> Self {
        //scale size, translate by (size/2)*(1-scale) to keep center in place
        Self {
            size: self.size * scale,
            offset: self.offset + self.size * 0.5 * (1.0 - scale),
        }
    }
    //maintains center of mass, expand by fixed amount
    pub fn expand(self, expansion: Vec3) -> Self {
        Self {
            size: self.size + expansion,
            offset: self.offset - expansion * 0.5,
        }
    }
    pub fn min(self) -> Vec3 {
        self.offset
    }
    pub fn world_min(self, pos: Vec3) -> Vec3 {
        self.min() + pos
    }
    pub fn max(self) -> Vec3 {
        self.offset + self.size
    }
    pub fn world_max(self, pos: Vec3) -> Vec3 {
        self.max() + pos
    }
    pub fn center(self) -> Vec3 {
        self.min() + self.size / 2.0
    }
    pub fn world_center(self, pos: Vec3) -> Vec3 {
        self.world_min(pos) + self.size / 2.0
    }
    pub fn from_block(physics: &BlockPhysics) -> Option<Self> {
        match physics {
            BlockPhysics::Empty => None,
            BlockPhysics::Solid => Some(Aabb {
                size: Vec3::splat(1.0),
                offset: Vec3::ZERO,
            }),
            BlockPhysics::Aabb(col) => Some(*col),
        }
    }

    pub fn intersects_point(self, my_pos: Vec3, point_pos: Vec3) -> bool {
        let min = self.world_min(my_pos);
        let max = self.world_max(my_pos);
        (point_pos.x >= min.x && point_pos.y >= min.y && point_pos.z >= min.z)
            && (point_pos.x < max.x && point_pos.y < max.y && point_pos.z < max.z)
    }
    pub fn intersects_aabb(self, my_pos: Vec3, other: Aabb, other_pos: Vec3) -> bool {
        let my_min = self.world_min(my_pos);
        let my_max = self.world_max(my_pos);
        let other_min = other.world_min(other_pos);
        let other_max = other.world_max(other_pos);
        (my_min.x < other_max.x && my_max.x > other_min.x)
            && (my_min.y < other_max.y && my_max.y > other_min.y)
            && (my_min.z < other_max.z && my_max.z > other_min.z)
    }

    //I had a lot of issues getting swept collision working, expect a lot of comments
    //returns (time, hit point, normal)
    pub fn sweep_ray(
        self,
        my_pos: Vec3,
        ray_start: Vec3,
        ray_delta: Vec3,
    ) -> Option<(f32, Vec3, crate::util::Direction)> {
        //assume start isn't inside for now
        let my_min = self.world_min(my_pos);
        let my_max = self.world_max(my_pos);

        //get times for intersection on each axis
        let mut t_near = (my_min - ray_start) / ray_delta;
        let mut t_far = (my_max - ray_start) / ray_delta;

        if t_near.x.is_nan()
            || t_near.y.is_nan()
            || t_near.z.is_nan()
            || t_far.x.is_nan()
            || t_far.y.is_nan()
            || t_far.z.is_nan()
        {
            return None;
        }

        //sort times, make sure that near is closer than far
        if t_near.x > t_far.x {
            std::mem::swap(&mut t_near.x, &mut t_far.x);
        }
        if t_near.y > t_far.y {
            std::mem::swap(&mut t_near.y, &mut t_far.y);
        }
        if t_near.z > t_far.z {
            std::mem::swap(&mut t_near.z, &mut t_far.z);
        }

        let t_hit_near = t_near.x.max(t_near.y).max(t_near.z);
        let t_hit_far = t_far.x.min(t_far.y).min(t_far.z);

        //no collision if far point is behind the ray origin
        if t_hit_far < 0.0 || t_hit_far < t_hit_near {
            return None;
        }

        //we have a collision!
        let hit_point = ray_start + t_hit_near * ray_delta;
        Some((
            t_hit_near,
            hit_point,
            if t_near.x > t_near.y && t_near.x > t_near.z {
                //hit on x axis
                if ray_delta.x < 0.0 {
                    crate::util::Direction::PosX
                } else {
                    crate::util::Direction::NegX
                }
            } else if t_near.y > t_near.x && t_near.y > t_near.z {
                //hit on y axis
                if ray_delta.y < 0.0 {
                    crate::util::Direction::PosY
                } else {
                    crate::util::Direction::NegY
                }
            } else {
                //hit on z axis
                if ray_delta.z < 0.0 {
                    crate::util::Direction::PosZ
                } else {
                    crate::util::Direction::NegZ
                }
            },
        ))
    }

    //I had a lot of issues getting swept collision working, expect a lot of comments
    //returns (time, hit point, normal)
    pub fn sweep_rect(
        self,
        my_pos: Vec3,
        my_v: Vec3,
        other: Aabb,
        other_pos: Vec3,
    ) -> Option<(f32, Vec3, crate::util::Direction)> {
        //assume not already in contact for now
        if my_v == Vec3::ZERO {
            return None;
        }

        //expand other rectangle by my size/2 in each direction so that we can just trace the center
        let other_expanded = other.expand(self.size);
        if let Some((time, contact_point, normal)) =
            other_expanded.sweep_ray(other_pos, self.world_center(my_pos), my_v)
        {
            if time >= 0.0 && time <= 1.0 {
                return Some((time, contact_point, normal));
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
}

#[derive(Component, Copy, Clone, Default)]
pub struct IgnoreTerrainCollision;

fn move_and_slide(
    mut objects: Query<
        (
            &mut Transform,
            &mut Velocity,
            &Acceleration,
            &mut CollidingDirections,
            &Aabb,
        ),
        Without<IgnoreTerrainCollision>,
    >,
    block_physics: Query<&BlockPhysics>,
    mut block_gizmos: ResMut<DebugBlockHitboxes>,
    level: Res<Level>,
    debug_state: Res<State<DebugUIState>>,
) {
    block_gizmos.blocks.clear();
    block_gizmos.hit_blocks.clear();
    let mut resolution_buffer: Vec<(f32, BlockCoord, Aabb)> = Vec::with_capacity(32);
    for (mut tf, mut v, a, mut directions, col) in objects.iter_mut() {
        resolution_buffer.clear();
        directions.0 = DirectionFlags::default();
        let target_pos = tf.translation+v.0;
        let bounding_min = tf.translation.min(target_pos);
        let bounding_max = tf.translation.max(target_pos);
        //all the blocks we can overlap with are in the bounding rectangle of current position and target position
        //add 1 in each direction to avoid issues with "precision"
        let overlaps = level.get_blocks_in_volume(BlockVolume::new_inclusive(
            BlockCoord::from(col.world_min(bounding_min))-BlockCoord::new(1,1,1),
            BlockCoord::from(col.world_max(bounding_max))+BlockCoord::new(1,1,1),
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

        //get all collision we need to resolve, sort in order of time, resolve all
        for (c, p) in overlaps_iter {
            if let Some(block_aabb) = Aabb::from_block(p) {
                if let Some((t, _, _)) =
                    col.sweep_rect(tf.translation, v.0, block_aabb, c.to_vec3())
                {
                    resolution_buffer.push((t, c, block_aabb));
                }
            }
        }

        //we count NaN time as no collision, so this is ok (all other floats are comparable)
        resolution_buffer.sort_unstable_by(|(t1, _, _), (t2, _, _)| {
            t1.partial_cmp(t2).unwrap_or(std::cmp::Ordering::Equal)
        });

        //resolve collisions
        for (_, block_pos, block_aabb) in resolution_buffer.drain(..) {
            if let Some((t, contact_point, contact_normal)) =
                col.sweep_rect(tf.translation, v.0, block_aabb, block_pos.to_vec3())
            {
                if *debug_state == DebugUIState::Shown {
                    block_gizmos.hit_blocks.insert(block_pos);
                    info!(
                        "block collision: ({:?},{:?},{:?})",
                        t, contact_point, contact_normal
                    );
                }
                //prevent clipping into blocks due to floating point imprecision
                let t_adj = if t < 0.0001 { 0.0 } else { t };
                let normal = contact_normal.to_vec3();
                //stop before penetrating the other box
                let v_abs = v.abs();
                v.0 += normal * v_abs * (1.0 - t_adj);
            }
        }

        tf.translation += v.0;
    }
}
