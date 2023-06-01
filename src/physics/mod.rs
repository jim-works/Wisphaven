mod level_physics;
pub use level_physics::*;

mod physics_objects;
pub use physics_objects::*;

use bevy::prelude::*;

use crate::world::LevelSystemSet;
use bevy_rapier3d::prelude::*;

pub struct PhysicsPlugin;

const SPAWN_CHUNK_TIME_BUDGET_COUNT: u32 = 1000;
pub const GRAVITY: f32 = -7.5;

pub const PLAYER_GROUP: u32 = 1 << 0;
pub const TERRAIN_GROUP: u32 = 1 << 1;
pub const JUMPABLE_GROUP: u32 = 1 << 2;
pub const ACTOR_GROUP: u32 = 1 << 3;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            //.add_plugin(RapierDebugRenderPlugin::default())
            .insert_resource(GeneratePhysicsTimer{timer: Timer::from_seconds(0.25, TimerMode::Repeating)})
            .add_systems((level_physics::queue_gen_physics,
                level_physics::poll_gen_physics_queue).in_set(LevelSystemSet::Main))
            .add_startup_system(configure_physics);
    }
}

fn configure_physics(
    mut config: ResMut<RapierConfiguration>
) {
    config.gravity = Vec3::new(0.0,GRAVITY,0.0);
}