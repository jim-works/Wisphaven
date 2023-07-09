pub use bevy::prelude::*;
use bevy::utils::HashMap;

use std::fs;
use std::path::PathBuf;

use crate::mesher::TerrainTexture;
use crate::serialization::LoadingBlocks;
use crate::serialization::db::LevelDB;
use crate::world::blocks::tnt::TNTBlock;
use crate::world::{LevelLoadState, BlockName, BlockResources, BlockRegistry, BlockPhysics, UsableBlock, NamedBlockMesh};
use crate::world::{events::CreateLevelEvent, settings::Settings, Level};

use super::BlockTextureMap;
use super::state::BlockTypesLoaded;

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

pub fn start_loading_blocks (
    assets: Res<AssetServer>,
    settings: Res<Settings>,
    mut commands: Commands,
) {
    let block_scenes: Vec<Handle<DynamicScene>> = assets.load_folder(settings.block_type_path.as_path())
                                            .into_iter()
                                            .flat_map(|v| v.into_iter().map(|t| t.typed()))
                                            .collect();
    if block_scenes.len() == 0 {
        warn!("No blocks found at {}", settings.block_type_path.as_path().display());
        return;
    }
    info!("Spawning {} blocks scenes", block_scenes.len());
    for block_scene in block_scenes {
        commands.spawn(
(DynamicSceneBundle {
                scene: block_scene,
                ..default()
            },
            Name::new("blocks"),
            LoadingBlocks
        ));
    }
}

pub fn load_block_registry(
    mut commands: Commands,
    texture_map: Res<BlockTextureMap>,
    loading_blocks: Query<(Entity, Option<&Children>), With<LoadingBlocks>>,
    block_name_query: Query<&BlockName>,
    name_resolution_query: Query<&NamedBlockMesh>,
    mut progress: ResMut<BlockTypesLoaded>
) {
    //make sure there are no still loading block scenes before we make the registry
    if progress.0 || loading_blocks.iter().any(|(_, opt_children)| opt_children.is_none()) {
        return;
    }
    let mut registry = BlockRegistry::default();
    for (scene_entity, children) in loading_blocks.iter() {
        info!("Loading block scene");
        commands.entity(scene_entity).remove::<LoadingBlocks>();
        for child in children.unwrap() {
            //do name resolution
            if let Ok(named_mesh) = name_resolution_query.get(*child) {
                commands.entity(*child)
                    .insert(named_mesh.clone().to_block_mesh(texture_map.as_ref()))
                    .remove::<NamedBlockMesh>();
            }
            match block_name_query.get(*child) {
                Ok(name) => registry.add_basic(name.clone(), *child, &mut commands),
                Err(e) => warn!("Block doesn't have a name! Error {:?}", e),
            }
            
        }
    }
    // registry.create_basic(BlockName::core("grass"), NamedBlockMesh::MultiTexture(["grass_side.png".into(), "grass_top.png".into(), "grass_side.png".into(),"grass_side.png".into(),"dirt.png".into(),"grass_side.png".into()]).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    // registry.create_basic(BlockName::core("dirt"), NamedBlockMesh::Uniform("dirt.png".into()).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    // registry.create_basic(BlockName::core("stone"), NamedBlockMesh::Uniform("stone.png".into()).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    // registry.create_basic(BlockName::core("log"), NamedBlockMesh::MultiTexture(["log_side.png".into(), "log_top.png".into(), "log_side.png".into(),"log_side.png".into(),"log_top.png".into(),"log_side.png".into()]).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    // registry.create_basic(BlockName::core("leaves"), NamedBlockMesh::Uniform("leaves.png".into()).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    // registry.create_basic(BlockName::core("log slab"), NamedBlockMesh::BottomSlab(0.5, ["log_side.png".into(), "log_top.png".into(), "log_side.png".into(),"log_side.png".into(),"log_top.png".into(),"log_side.png".into()]).to_block_mesh(texture_map.as_ref()), BlockPhysics::BottomSlab(0.5), &mut commands);
    // let id = registry.create_basic(BlockName::core("tnt"), NamedBlockMesh::MultiTexture(["tnt_side.png".into(), "tnt_top.png".into(), "tnt_side.png".into(),"tnt_side.png".into(),"tnt_top.png".into(),"tnt_side.png".into()]).to_block_mesh(texture_map.as_ref()), BlockPhysics::Solid, &mut commands);
    // commands.entity(id).insert((TNTBlock {explosion_strength: 10.0}, UsableBlock, NamedBlockMesh::MultiTexture(["tnt_side.png".into(), "tnt_top.png".into(), "tnt_side.png".into(),"tnt_side.png".into(),"tnt_top.png".into(),"tnt_side.png".into()])));

    info!("Finished loading {} block types", registry.id_map.len());
    commands.insert_resource(BlockResources {registry: std::sync::Arc::new(registry)});
    progress.0 = true;
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
