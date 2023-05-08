mod mesher;
pub use mesher::*;

use bevy::prelude::*;

use crate::world::LevelSystemSet;

pub struct MesherPlugin;

const QUEUE_MESH_TIME_BUDGET_MS: u128 = 10;
const SPAWN_MESH_TIME_BUDGET_COUNT: u32 = 1000;


impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems((mesher::poll_mesh_queue,mesher::queue_meshing).in_set(LevelSystemSet::Main));
    }
}