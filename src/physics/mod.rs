use bevy::{prelude::*, transform::TransformSystem};

use crate::{ui::debug::DebugDrawTransform, world::LevelLoadState};
use bevy_rapier3d::prelude::*;

#[cfg(test)]
mod tests;

pub mod collision;
pub mod movement;

const SPAWN_CHUNK_TIME_BUDGET_COUNT: u32 = 1000;
pub const GRAVITY: Vec3 = Vec3::new(0., -10.0, 0.);

pub const PLAYER_GROUP: u32 = 1 << 0;
pub const TERRAIN_GROUP: u32 = 1 << 1;
pub const ACTOR_GROUP: u32 = 1 << 3;

pub const TPS: f64 = 64.0;
pub const TICK_SCALE: f64 = 64.0/TPS;

//run in fixed update
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PhysicsSystemSet {
    Main, //all user code should run here
    UpdatePositionVelocity,
    CollisionResolution,
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RapierPhysicsPlugin::<NoUserData>::default(),
            movement::MovementPlugin,
            collision::CollisionPlugin,
        ))
        .insert_resource(Time::<Fixed>::from_hz(TPS))
        .configure_sets(
            FixedUpdate,
            (
                PhysicsSystemSet::Main,
                PhysicsSystemSet::UpdatePositionVelocity,
                PhysicsSystemSet::CollisionResolution,
            )
                .chain().run_if(in_state(LevelLoadState::Loaded)),
        )
        .configure_sets(FixedUpdate, TransformSystem::TransformPropagate.after(PhysicsSystemSet::CollisionResolution))
        .add_systems(Startup, configure_physics);
    }
}

fn configure_physics(mut config: ResMut<RapierConfiguration>) {
    config.gravity = GRAVITY;
}

#[derive(Bundle)]
pub struct PhysicsObjectBundle {
    pub rigidbody: RigidBody,
    pub ccd: Ccd,
    pub locked_axes: LockedAxes,
    pub collider: Collider,
    pub external_impulse: ExternalImpulse,
    pub velocity: Velocity,
    pub collision_groups: CollisionGroups,
    pub debug_draw_transform: DebugDrawTransform,
}

#[derive(Bundle, Default)]
pub struct PhysicsBundle {
    pub velocity: movement::Velocity,
    pub acceleration: movement::Acceleration,
    pub gravity: movement::GravityMult,
    pub collider: collision::Collider,
    pub colliding_directions: collision::CollidingDirections,
    pub desired_position: collision::DesiredPosition,
    pub friction: collision::Friction,
}

impl Default for PhysicsObjectBundle {
    fn default() -> Self {
        PhysicsObjectBundle {
            rigidbody: RigidBody::Dynamic,
            ccd: Ccd::disabled(),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            collider: Collider::capsule(Vec3::ZERO, Vec3::new(0.0, 1.8, 0.0), 0.4),
            external_impulse: ExternalImpulse::default(),
            velocity: Velocity::default(),
            collision_groups: CollisionGroups::new(
                Group::from_bits_truncate(ACTOR_GROUP),
                Group::all(),
            ),
            debug_draw_transform: DebugDrawTransform,
        }
    }
}

pub fn shape_intersects_with_actors(
    ctx: &Res<RapierContext>,
    shape_pos: Vec3,
    shape_rot: Quat,
    shape: &Collider,
    exclude: Option<Entity>,
    callback: impl FnMut(Entity) -> bool,
) {
    ctx.intersections_with_shape(
        shape_pos,
        shape_rot,
        shape,
        QueryFilter {
            groups: Some(CollisionGroups::new(
                Group::ALL,
                Group::from_bits_truncate(ACTOR_GROUP),
            )),
            exclude_collider: exclude,
            ..default()
        },
        callback,
    );
}
