pub use bevy::prelude::*;

use std::fs;

use crate::serialization::db::LevelDB;
use crate::world::{LevelLoadState, BlockName, BlockMesh, BlockResources, BlockRegistry, BlockPhysics};
use crate::world::{events::CreateLevelEvent, settings::Settings, Level};

pub fn load_block_registry(
    mut commands: Commands
) {
    let mut registry = BlockRegistry::default();
    registry.create_basic(BlockName::core("grass"), BlockMesh::MultiTexture([1,0,1,1,3,1]), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("dirt"), BlockMesh::Uniform(3), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("stone"), BlockMesh::Uniform(2), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("log"), BlockMesh::MultiTexture([5,6,5,5,6,5]), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("leaves"), BlockMesh::Uniform(7), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("log slab"), BlockMesh::BottomSlab(0.5, [5,6,5,5,6,5]), BlockPhysics::BottomSlab(0.5), &mut commands);
    commands.insert_resource(BlockResources {registry: std::sync::Arc::new(registry)});
}

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
        let db = LevelDB::new(settings.env_path.join(event.name.to_owned() + ".db").as_path());
        match db {
            Ok(mut db) => {
                if let Some(err) = db.immediate_create_chunk_table() {
                    error!("Error creating chunk table: {:?}", err);
                    return;
                }
                commands.insert_resource(db);
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
