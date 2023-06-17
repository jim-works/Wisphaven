pub use bevy::prelude::*;

use heed::EnvOpenOptions;
use std::fs;

use crate::world::chunk::ChunkCoord;
use crate::world::LevelLoadState;
use crate::world::{events::CreateLevelEvent, settings::Settings, Level};

use super::{ChunkDB, ChunkSaveFormat, HeedEnv};

pub fn on_level_created(
    mut reader: EventReader<CreateLevelEvent>,
    settings: Res<Settings>,
    mut next_state: ResMut<NextState<LevelLoadState>>,
    mut commands: Commands,
) {
    const MAX_DBS: u32 = 1;
    info!("on_level_created");
    for event in reader.iter() {
        fs::create_dir_all(settings.env_path.as_path()).unwrap();
        let env = EnvOpenOptions::new()
            .max_dbs(MAX_DBS)
            .open(settings.env_path.as_path())
            .unwrap();
        let db = env.open_database::<ChunkCoord, ChunkSaveFormat>(Some(&event.name));
        match db {
            Ok(db_opt) => {
                let db = match db_opt {
                    Some(db) => db,
                    None => match env.create_database::<ChunkCoord, ChunkSaveFormat>(Some(&event.name)) {
                        Ok(db) => db,
                        Err(e) => {
                            error!("couldn't create world db {}", e);
                            panic!();
                        }
                    }
                };
                commands.insert_resource(HeedEnv(env));
                commands.insert_resource(ChunkDB(db));
                commands.insert_resource(Level::new(
                    event.name,
                    settings.init_loader.lod_levels.try_into().unwrap(),
                ));
                next_state.set(LevelLoadState::Loading);
                info!("in state loading!");
            }
            Err(e) => {
                error!("couldn't open db {}", e);
            }
        }
    }
}
