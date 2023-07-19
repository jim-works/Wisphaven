mod level_physics;
pub use level_physics::*;

use bevy::prelude::*;

use crate::world::LevelSystemSet;
use bevy_rapier3d::prelude::*;

pub struct PhysicsPlugin;

const SPAWN_CHUNK_TIME_BUDGET_COUNT: u32 = 1000;
pub const GRAVITY: f32 = -10.0;

pub const PLAYER_GROUP: u32 = 1 << 0;
pub const TERRAIN_GROUP: u32 = 1 << 1;
pub const JUMPABLE_GROUP: u32 = 1 << 2;
pub const ACTOR_GROUP: u32 = 1 << 3;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
            //.add_plugins(RapierDebugRenderPlugin::default())
            .add_systems(
                Update,
                (
                    level_physics::queue_gen_physics,
                    level_physics::poll_gen_physics_queue,
                )
                    .in_set(LevelSystemSet::AfterLoadingAndMain),
            )
            .add_systems(Startup, configure_physics);
    }
}

fn configure_physics(mut config: ResMut<RapierConfiguration>) {
    config.gravity = Vec3::new(0.0, GRAVITY, 0.0);
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
}

impl Default for PhysicsObjectBundle {
    fn default() -> Self {
        PhysicsObjectBundle {
            rigidbody: RigidBody::Dynamic,
            ccd: Ccd::enabled(),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            collider: Collider::capsule(Vec3::ZERO, Vec3::new(0.0, 1.8, 0.0), 0.4),
            external_impulse: ExternalImpulse::default(),
            velocity: Velocity::default(),
            collision_groups: CollisionGroups::new(
                Group::from_bits_truncate(ACTOR_GROUP),
                Group::all(),
            ),
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
            groups: Some(CollisionGroups::new(Group::ALL, Group::from_bits_truncate(ACTOR_GROUP))),
            exclude_collider: exclude,
            ..default()
        },
        callback,
    );
}
