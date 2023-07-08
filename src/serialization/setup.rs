pub use bevy::prelude::*;
use bevy::utils::HashMap;

use std::fs;
use std::path::{Path, PathBuf};

use crate::mesher::TerrainTexture;
use crate::serialization::db::LevelDB;
use crate::world::blocks::tnt::TNTBlock;
use crate::world::{LevelLoadState, BlockName, BlockMesh, BlockResources, BlockRegistry, BlockPhysics, UsableBlock, NamedBlockMesh};
use crate::world::{events::CreateLevelEvent, settings::Settings, Level};

use super::BlockTextureMap;

pub fn load_settings() -> Settings {
    Settings::default()
}

//begins loading the terrain texture images and creates the filename->texture id map
pub fn load_terrain_texture(
    mut commands: Commands,
    assets: Res<AssetServer>,
    settings: Res<Settings>,
) {
    let textures: Vec<Handle<Image>> = assets.load_folder(settings.block_tex_path.as_path())
                                            .into_iter()
                                            .flat_map(|v| v.into_iter().map(|t| t.typed()))
                                            .collect();
    if textures.len() == 0 {
        warn!("No block textures found at {}", settings.block_tex_path.as_path().display());
        return;
    }
    
    let mut names = HashMap::new();
    for (i, texture) in textures.iter().enumerate() {
        //`get_handle_path` returns "textures/blocks/folder/name.png"
        //this removes the "textures/blocks" to leave us with "folder/name.png"
        let texture_name: PathBuf = assets.get_handle_path(texture).unwrap().path().strip_prefix("textures/blocks").unwrap().to_path_buf();
        info!("Loaded texture name {} with id {}", texture_name.display(), i);
        names.insert(texture_name, i as u32);
    }
    commands.insert_resource(BlockTextureMap(names));
    commands.insert_resource(TerrainTexture(textures));
}

pub fn load_block_registry(
    mut commands: Commands,
    texture_map: Res<BlockTextureMap>
) {
    let mut registry = BlockRegistry::default();
    
    
    registry.create_basic(BlockName::core("grass"), NamedBlockMesh::MultiTexture(["grass_side.png".into(), "grass_top.png".into(), "grass_side.png".into(),"grass_side.png".into(),"dirt.png".into(),"grass_side.png".into()]).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("dirt"), NamedBlockMesh::Uniform("dirt.png".into()).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("stone"), NamedBlockMesh::Uniform("stone.png".into()).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("log"), NamedBlockMesh::MultiTexture(["log_side.png".into(), "log_top.png".into(), "log_side.png".into(),"log_side.png".into(),"log_top.png".into(),"log_side.png".into()]).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("leaves"), NamedBlockMesh::Uniform("leaves.png".into()).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    registry.create_basic(BlockName::core("log slab"), NamedBlockMesh::BottomSlab(0.5, ["log_side.png".into(), "log_top.png".into(), "log_side.png".into(),"log_side.png".into(),"log_top.png".into(),"log_side.png".into()]).to_block_mesh(texture_map.as_ref()), BlockPhysics::BottomSlab(0.5), &mut commands);
    let id = registry.create_basic(BlockName::core("tnt"), NamedBlockMesh::MultiTexture(["tnt_side.png".into(), "tnt_top.png".into(), "tnt_side.png".into(),"tnt_side.png".into(),"tnt_top.png".into(),"tnt_side.png".into()]).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    commands.entity(id).insert((TNTBlock {explosion_strength: 10.0}, UsableBlock));

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
