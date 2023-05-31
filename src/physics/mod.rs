mod level_physics;
pub use level_physics::*;

use bevy::prelude::*;

use crate::world::LevelSystemSet;

pub struct PhysicsPlugin;

const SPAWN_CHUNK_TIME_BUDGET_COUNT: u32 = 1000;


impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(GeneratePhysicsTimer{timer: Timer::from_seconds(0.25, TimerMode::Repeating)})
            .add_systems((level_physics::queue_gen_physics,level_physics::poll_gen_physics_queue).in_set(LevelSystemSet::Main));
    }
}