pub use bevy::prelude::*;

use std::fs;
use heed::EnvOpenOptions;

use crate::world::LevelLoadState;
use crate::world::chunk::ChunkCoord;
use crate::world::{events::CreateLevelEvent, Level, settings::Settings};

use super::{ChunkSaveFormat, ChunkDB, HeedEnv};

pub fn on_level_created (
    mut reader: EventReader<CreateLevelEvent>,
    settings: Res<Settings>,
    mut next_state: ResMut<NextState<LevelLoadState>>,
    mut commands: Commands
) {
    for event in reader.iter() {
        fs::create_dir_all(settings.env_path.as_path()).unwrap();
        let env = EnvOpenOptions::new().open(settings.env_path.as_path()).unwrap();
        let db = env.create_database::<ChunkCoord, ChunkSaveFormat>(Some(&event.name)).unwrap();
        commands.insert_resource(HeedEnv(env));
        commands.insert_resource(ChunkDB(db));
        commands.insert_resource(Level::new(
            event.name.clone(),
            settings.init_loader.lod_levels.try_into().unwrap()
        ));
        next_state.set(LevelLoadState::Loading);
    }
}