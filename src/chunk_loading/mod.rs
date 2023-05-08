mod entity_loader;

pub use entity_loader::ChunkLoader;

use bevy::prelude::*;

use crate::world::LevelSystemSet;

use self::entity_loader::DespawnChunkEvent;

pub struct ChunkLoaderPlugin;


impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(entity_loader::do_loading.in_set(LevelSystemSet::Main))
            .add_system(entity_loader::despawn_chunks.in_set(LevelSystemSet::Despawn))
            .add_event::<DespawnChunkEvent>();
    }
}