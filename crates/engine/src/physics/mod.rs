use bevy::{prelude::*, transform::TransformSystem};
use movement::Restitution;

use crate::{debug::DebugDrawTransform, world::LevelLoadState};

use self::{collision::IgnoreTerrainCollision, movement::Drag};

pub mod collision;
pub mod grapple;
pub mod movement;
pub mod query;
pub mod spring;
mod test;

const SPAWN_CHUNK_TIME_BUDGET_COUNT: u32 = 1000;
pub const GRAVITY: Vec3 = Vec3::new(0., -10.0, 0.);

pub const TPS: f64 = 64.0;

//run in fixed update
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PhysicsSystemSet {
    Main, //all user code should run here
    ResetInterpolation,
    ProcessRaycasts,
    UpdatePosition,
    UpdateDerivatives,
}

//run in fixed update
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PhysicsLevelSet {
    Main,
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            movement::MovementPlugin,
            collision::CollisionPlugin,
            grapple::GrapplePlugin,
            spring::SpringPlugin,
        ))
        .insert_resource(Time::<Fixed>::from_hz(TPS))
        .configure_sets(
            FixedUpdate,
            (
                PhysicsSystemSet::Main,
                PhysicsSystemSet::ResetInterpolation,
                PhysicsSystemSet::ProcessRaycasts,
                PhysicsSystemSet::UpdatePosition,
                PhysicsSystemSet::UpdateDerivatives,
            )
                .chain(),
        )
        .configure_sets(
            FixedUpdate,
            PhysicsLevelSet::Main
                .in_set(PhysicsSystemSet::Main)
                .run_if(in_state(LevelLoadState::Loaded)),
        )
        .configure_sets(
            FixedUpdate,
            TransformSystem::TransformPropagate.after(PhysicsSystemSet::UpdatePosition),
        );
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
