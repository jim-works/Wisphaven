use bevy::{prelude::*, transform::TransformSystem};

use crate::{ui::debug::DebugDrawTransform, world::LevelLoadState};

use self::collision::IgnoreTerrainCollision;

pub mod collision;
pub mod movement;
pub mod query;

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

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            movement::MovementPlugin,
            collision::CollisionPlugin,
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
                .chain().run_if(in_state(LevelLoadState::Loaded)),
        )
        .configure_sets(FixedUpdate, TransformSystem::TransformPropagate.after(PhysicsSystemSet::UpdatePosition));
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
    pub ignore_terrain_collision: IgnoreTerrainCollision
}

#[derive(Bundle, Default)]
pub struct FrictionBundle {
    pub friction: collision::Friction,
    pub colliding_blocks: collision::CollidingBlocks,
}