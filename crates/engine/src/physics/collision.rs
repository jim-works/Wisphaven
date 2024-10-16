use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use util::{
    direction::{Direction, DirectionFlags},
    iterators::{Volume, VolumeContainer},
    *,
};

use crate::{
    debug::{DebugBlockHitboxes, DebugUIState},
    world::{BlockCoord, BlockPhysics, BlockType, Level},
};

use super::{movement::*, PhysicsLevelSet};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (move_and_slide, update_terrain_query_point).in_set(PhysicsLevelSet::Main),
        )
        .register_type::<Aabb>();
    }
}

#[derive(Component)]
pub struct Friction(pub f32);

impl Default for Friction {
    fn default() -> Self {
        Self(0.825)
    }
}

#[derive(Component, Default)]
pub struct CollidingBlocks {
    pub pos_x: Vec<(BlockCoord, Entity, BlockPhysics)>,
    pub pos_y: Vec<(BlockCoord, Entity, BlockPhysics)>,
    pub pos_z: Vec<(BlockCoord, Entity, BlockPhysics)>,
    pub neg_x: Vec<(BlockCoord, Entity, BlockPhysics)>,
    pub neg_y: Vec<(BlockCoord, Entity, BlockPhysics)>,
    pub neg_z: Vec<(BlockCoord, Entity, BlockPhysics)>,
}

impl CollidingBlocks {
    pub fn get(&self, direction: Direction) -> &Vec<(BlockCoord, Entity, BlockPhysics)> {
        match direction {
            Direction::PosX => &self.pos_x,
            Direction::PosY => &self.pos_y,
            Direction::PosZ => &self.pos_z,
            Direction::NegX => &self.neg_x,
            Direction::NegY => &self.neg_y,
            Direction::NegZ => &self.neg_z,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.pos_x.is_empty()
            && self.pos_y.is_empty()
            && self.pos_z.is_empty()
            && self.neg_x.is_empty()
            && self.neg_y.is_empty()
            && self.neg_z.is_empty()
    }
    pub fn get_mut(
        &mut self,
        direction: Direction,
    ) -> &mut Vec<(BlockCoord, Entity, BlockPhysics)> {
        match direction {
            Direction::PosX => &mut self.pos_x,
            Direction::PosY => &mut self.pos_y,
            Direction::PosZ => &mut self.pos_z,
            Direction::NegX => &mut self.neg_x,
            Direction::NegY => &mut self.neg_y,
            Direction::NegZ => &mut self.neg_z,
        }
    }
    pub fn clear(&mut self) {
        self.pos_x.clear();
        self.pos_y.clear();
        self.pos_z.clear();
        self.neg_x.clear();
        self.neg_y.clear();
        self.neg_z.clear();
    }
    pub fn for_each_dir(
        &self,
        mut f: impl FnMut(Direction, &Vec<(BlockCoord, Entity, BlockPhysics)>),
    ) {
        f(Direction::PosX, &self.pos_x);
        f(Direction::PosY, &self.pos_y);
        f(Direction::PosZ, &self.pos_z);
        f(Direction::NegX, &self.neg_x);
        f(Direction::NegY, &self.neg_y);
        f(Direction::NegZ, &self.neg_z);
    }
    pub fn iter(&self) -> impl Iterator<Item = &(BlockCoord, Entity, BlockPhysics)> {
        self.pos_x
            .iter()
            .chain(self.pos_y.iter())
            .chain(self.pos_z.iter())
            .chain(self.neg_x.iter())
            .chain(self.neg_y.iter())
            .chain(self.neg_z.iter())
    }
    pub fn push(&mut self, direction: Direction, elem: (BlockCoord, Entity, BlockPhysics)) {
        self.get_mut(direction).push(elem);
    }
}

//from offset to offset + size
#[derive(Component, Clone, Copy, PartialEq, Default, Reflect, Debug, Serialize, Deserialize)]
#[reflect(Component, FromWorld)]
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
    pub fn centered(size: Vec3) -> Self {
        Self {
            size,
            offset: -size / 2.0,
        }
    }
    pub fn add_offset(self, offset: Vec3) -> Self {
        Self::new(self.size, self.offset + offset)
    }
    pub fn add_size(self, size: Vec3) -> Self {
        Self::new(self.size + size, self.offset)
    }
    //maintains center of mass, scale by factor
    pub fn scale(self, scale: Vec3) -> Self {
        //scale size, translate by (size/2)*(1-scale) to keep center in place
        Self {
            size: self.size * scale,
            offset: self.offset + self.size * 0.5 * (1.0 - scale),
        }
    }
    //grows in all directions, maintaining center of mass
    pub fn grow(self, expansion: Vec3) -> Self {
        Self {
            size: self.size + expansion,
            offset: self.offset - expansion * 0.5,
        }
    }
    //moves the minimum corner of the aabb while keeping the maximum corner in place
    pub fn move_min(self, delta: Vec3) -> Self {
        self.add_offset(delta).add_size(-delta)
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
    pub fn to_volume(self, offset: Vec3) -> Volume {
        let max_corner = self.world_max(offset);
        Volume::new_inclusive(
            IVec3::my_from(self.world_min(offset)),
            IVec3::new(
                max_corner.x.floor() as i32,
                max_corner.y.floor() as i32,
                max_corner.z.floor() as i32,
            ),
        )
    }
    pub fn to_block_volume(self, pos: Vec3) -> Volume {
        Volume::new_inclusive(self.world_min(pos).my_into(), self.world_max(pos).my_into())
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
    pub fn intersects_block(
        self,
        my_pos: Vec3,
        other: &BlockPhysics,
        other_pos: BlockCoord,
    ) -> bool {
        if let Some(other_aabb) = Aabb::from_block(other) {
            self.intersects_aabb(my_pos, other_aabb, other_pos.to_vec3())
        } else {
            false
        }
    }

    //I had a lot of issues getting swept collision working, expect a lot of comments
    //returns (time, hit point, normal)
    pub fn sweep_ray(
        self,
        my_pos: Vec3,
        ray_start: Vec3,
        ray_delta: Vec3,
    ) -> Option<(f32, Vec3, Direction)> {
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
                    Direction::PosX
                } else {
                    Direction::NegX
                }
            } else if t_near.y > t_near.x && t_near.y > t_near.z {
                //hit on y axis
                if ray_delta.y < 0.0 {
                    Direction::PosY
                } else {
                    Direction::NegY
                }
            } else {
                //hit on z axis
                if ray_delta.z < 0.0 {
                    Direction::PosZ
                } else {
                    Direction::NegZ
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
    ) -> Option<(f32, Vec3, Direction)> {
        //assume not already in contact for now
        if my_v == Vec3::ZERO {
            return None;
        }

        //expand other rectangle by my size/2 in each direction so that we can just trace the center
        let other_expanded = other.grow(self.size);
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

// updates CollidingDirections and optionally colliding blocks only off the entities translation without doing collision logic.
// when the translation is inside a collidable block, all directions will be colliding with that one block.
#[derive(Component, Copy, Clone, Default)]
pub struct TerrainQueryPoint;

fn move_and_slide(
    mut objects: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut CollidingDirections,
            &Aabb,
            &Restitution,
            Option<&mut CollidingBlocks>,
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
    for (mut tf, mut v, mut directions, col, restitution, mut opt_col_blocks) in objects.iter_mut()
    {
        if let Some(ref mut col_blocks) = opt_col_blocks {
            col_blocks.clear();
        }
        resolution_buffer.clear();
        directions.0 = DirectionFlags::default();
        let target_pos = tf.translation + v.0;
        let bounding_min = tf.translation.min(target_pos);
        let bounding_max = tf.translation.max(target_pos);
        //all the blocks we can overlap with are in the bounding rectangle of current position and target position
        //add 1 in each direction to avoid issues with "precision"
        let overlaps = level.get_blocks_in_volume(Volume::new_inclusive(
            (BlockCoord::from(col.world_min(bounding_min)) - BlockCoord::new(1, 1, 1)).into(),
            (BlockCoord::from(col.world_max(bounding_max)) + BlockCoord::new(1, 1, 1)).into(),
        ));
        let overlaps_iter = overlaps.iter().filter_map(|(coord, block)| {
            block
                .and_then(|b| b.entity())
                .and_then(|e| block_physics.get(e).ok().and_then(|p| Some((coord, p, e))))
        });
        if *debug_state == DebugUIState::Shown {
            let gizmos_iter = overlaps.iter().map(|(coord, block)| {
                (
                    BlockCoord::from(coord),
                    block
                        .and_then(|b| b.entity())
                        .and_then(|e| block_physics.get(e).ok())
                        .and_then(|p| Some(p.clone())),
                )
            });
            block_gizmos.blocks.extend(gizmos_iter);
        }

        //get all collision we need to resolve, sort in order of time, resolve all
        for (c, p, e) in overlaps_iter {
            if let Some(block_aabb) = Aabb::from_block(p) {
                if let Some((t, _, normal)) =
                    col.sweep_rect(tf.translation, v.0, block_aabb, c.as_vec3())
                {
                    //do on collision events here, we won't actually resolve all these collisions
                    //  (resolving some will make the object not collide with others)
                    if *debug_state == DebugUIState::Shown {
                        block_gizmos.hit_blocks.insert(c.into());
                    }
                    if let Some(ref mut col_blocks) = opt_col_blocks {
                        col_blocks.push(normal, (c.into(), e, p.clone()));
                    }
                    resolution_buffer.push((t, c.into(), block_aabb));
                    directions.0.set(normal.opposite().into(), true);
                }
            }
        }

        //we count NaN time as no collision, so this is ok (all other floats are comparable)
        resolution_buffer.sort_unstable_by(|(t1, _, _), (t2, _, _)| {
            debug_assert!(!t1.is_nan() && !t2.is_nan());
            t1.partial_cmp(t2).unwrap_or(std::cmp::Ordering::Equal)
        });

        //resolve collisions
        for (_, block_pos, block_aabb) in resolution_buffer.drain(..) {
            if let Some((t, _, contact_normal)) =
                col.sweep_rect(tf.translation, v.0, block_aabb, block_pos.to_vec3())
            {
                //prevent clipping into blocks due to floating point imprecision
                let t_adj = if t < 0.0001 { 0.0 } else { t };
                let normal = contact_normal.to_vec3();
                //stop before penetrating the other box
                let v_abs = v.abs();
                v.0 += normal * v_abs * (1.0 - t_adj) + normal * restitution.0;
            }
        }
        tf.translation += v.0;
    }
}

fn update_terrain_query_point(
    mut objects: Query<
        (
            &GlobalTransform,
            &mut CollidingDirections,
            Option<&mut CollidingBlocks>,
        ),
        With<TerrainQueryPoint>,
    >,
    block_physics: Query<&BlockPhysics>,
    level: Res<Level>,
) {
    for (gtf, mut dir, mut opt_col_blocks) in objects.iter_mut() {
        if let Some(ref mut col_blocks) = opt_col_blocks {
            col_blocks.clear();
        }
        dir.0 = DirectionFlags::empty();
        let coord = gtf.translation().into();
        let Some(block) = level.get_block_entity(coord) else {
            continue;
        };
        let Ok(physics) = block_physics.get(block) else {
            continue;
        };
        dir.0 = DirectionFlags::all();
        if let Some(ref mut col_blocks) = opt_col_blocks {
            let elem = (coord, block, physics.clone());
            col_blocks.push(Direction::NegX, elem.clone());
            col_blocks.push(Direction::NegY, elem.clone());
            col_blocks.push(Direction::NegZ, elem.clone());
            col_blocks.push(Direction::PosX, elem.clone());
            col_blocks.push(Direction::PosY, elem.clone());
            col_blocks.push(Direction::PosZ, elem);
        }
    }
}

pub fn get_volume_from_collider(
    position: Vec3,
    collider: Aabb,
    level: &Level,
) -> VolumeContainer<BlockType> {
    level.get_blocks_in_volume(Volume::new_inclusive(
        (BlockCoord::from(collider.world_min(position)) - BlockCoord::new(1, 1, 1)).into(),
        (BlockCoord::from(collider.world_max(position)) + BlockCoord::new(1, 1, 1)).into(),
    ))
}
