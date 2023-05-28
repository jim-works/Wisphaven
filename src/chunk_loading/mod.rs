pub mod entity_loader;

pub use entity_loader::ChunkLoader;

use bevy::prelude::*;

use crate::world::LevelSystemSet;

use self::entity_loader::DespawnChunkEvent;

pub struct ChunkLoaderPlugin;


impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((entity_loader::do_loading,entity_loader::unload_all).in_set(LevelSystemSet::Main))
            .add_system(entity_loader::despawn_chunks.in_set(LevelSystemSet::Despawn))
            .add_event::<DespawnChunkEvent>();
    }
}