use ahash::{HashMap, HashSet};
use bevy::prelude::*;
use collision::BlockPhysics;
use interfaces::{components::DebugDrawTransform, scheduling::*};
use movement::Restitution;
use world::block::BlockCoord;

use self::{collision::IgnoreTerrainCollision, movement::Drag};

pub mod collision;
pub mod grapple;
pub mod interpolation;
pub mod movement;
pub mod query;
pub mod spring;
mod test;

const SPAWN_CHUNK_TIME_BUDGET_COUNT: u32 = 1000;
pub const GRAVITY: Vec3 = Vec3::new(0., -10.0, 0.);

pub const TPS: f64 = 64.0;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            movement::MovementPlugin,
            collision::CollisionPlugin,
            grapple::GrapplePlugin,
            spring::SpringPlugin,
            interpolation::InterpolationPlugin,
        ))
        .insert_resource(Time::<Fixed>::from_hz(TPS))
        .insert_resource(DebugBlockHitboxes::default());
    }
}
#[derive(Bundle, Default)]
pub struct PhysicsBundle {
    pub velocity: movement::Velocity,
    pub acceleration: movement::Acceleration,
    pub mass: movement::Mass,
    pub gravity: movement::GravityMult,
    pub collider: collision::Aabb,
    pub colliding_directions: collision::CollidingDirections,
    pub debug_draw_transform: DebugDrawTransform,
    pub friction: FrictionBundle,
    pub drag: Drag,
    pub restitution: Restitution,
}

#[derive(Bundle, Default)]
pub struct NoTerrainPhysicsBundle {
    pub velocity: movement::Velocity,
    pub acceleration: movement::Acceleration,
    pub mass: movement::Mass,
    pub gravity: movement::GravityMult,
    pub collider: collision::Aabb,
    pub colliding_directions: collision::CollidingDirections,
    pub debug_draw_transform: DebugDrawTransform,
    pub ignore_terrain_collision: IgnoreTerrainCollision,
}

#[derive(Bundle, Default)]
pub struct FrictionBundle {
    pub friction: collision::Friction,
    pub colliding_blocks: collision::CollidingBlocks,
}

#[derive(Resource, Default)]
pub struct DebugBlockHitboxes {
    pub blocks: HashMap<BlockCoord, Option<BlockPhysics>>,
    pub hit_blocks: HashSet<BlockCoord>,
}
