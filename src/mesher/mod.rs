mod mesher;
pub use mesher::*;

use bevy::prelude::*;

pub struct MesherPlugin;

const QUEUE_MESH_TIME_BUDGET_MS: u128 = 1;
const SPAWN_MESH_TIME_BUDGET_COUNT: u32 = 50;


impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(mesher::poll_mesh_queue)
            .add_system(mesher::queue_meshing);
    }
}