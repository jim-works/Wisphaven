pub use bevy::prelude::*;
use bevy::utils::HashMap;

use std::fs;
use std::path::PathBuf;

use crate::mesher::TerrainTexture;
use crate::serialization::LoadingBlocks;
use crate::serialization::db::{LevelDB, LevelDBErr};
use crate::serialization::queries::{CREATE_CHUNK_TABLE, CREATE_WORLD_INFO_TABLE, LOAD_WORLD_INFO, INSERT_WORLD_INFO};
use crate::world::{LevelLoadState, BlockName, BlockResources, BlockRegistry, NamedBlockMesh, BlockNameIdMap};
use crate::world::{events::CreateLevelEvent, settings::Settings, Level};

use super::{BlockTextureMap, LoadedToSavedBlockIdMap, SavedToLoadedBlockIdMap};

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
    block_resources: Option<Res<BlockResources>>
) {
    //make sure there are no still loading block scenes before we make the registry
    if block_resources.is_some() || loading_blocks.iter().any(|(_, opt_children)| opt_children.is_none()) {
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

    info!("Finished loading {} block types", registry.id_map.len());
    commands.insert_resource(BlockResources {registry: std::sync::Arc::new(registry)});
}

pub fn on_level_created(
    mut reader: EventReader<CreateLevelEvent>,
    settings: Res<Settings>,
    resources: Res<BlockResources>,
    mut next_state: ResMut<NextState<LevelLoadState>>,
    mut commands: Commands,
) {
    const MAX_DBS: u32 = 1;
    info!("on_level_created");
    if let Some(event) = reader.iter().next() {
        fs::create_dir_all(settings.env_path.as_path()).unwrap();
        let db = LevelDB::new(settings.env_path.join(event.name.to_owned() + ".db").as_path());
        match db {
            Ok(mut db) => {
                if let Some(err) = db.immediate_execute_command(|sql| sql.execute(CREATE_CHUNK_TABLE, [])) {
                    error!("Error creating chunk table: {:?}", err);
                    return;
                }
                if let Some(err) = db.immediate_execute_command(|sql| sql.execute(CREATE_WORLD_INFO_TABLE, [])) {
                    error!("Error creating world info table: {:?}", err);
                    return;
                }
                match db.immediate_execute_query(LOAD_WORLD_INFO, rusqlite::params!["block_palette"], |row| Ok(row.get(0)?)) {
                    Ok(data) => {
                        match create_block_id_maps_from_palette(&data, resources.registry.as_ref()) {
                            Some((mut saved_to_loaded, mut loaded_to_saved)) => {
                                //if we have new blocks that were not in the palette before, add them
                                let palette = create_palette_from_block_id_map(resources.registry.as_ref(), &mut saved_to_loaded, &mut loaded_to_saved);
                                if let Some (err) = db.immediate_execute_command(|sql| sql.execute(INSERT_WORLD_INFO, rusqlite::params!["block_palette", palette])) {
                                    error!("Error updating block palette! {:?}", err);
                                    return;
                                }
                                //put the updated maps in the world
                                commands.insert_resource(saved_to_loaded);
                                commands.insert_resource(loaded_to_saved);
                            },
                            None => {
                                error!("Couldn't create saved block id map!");
                                return;
                            },
                        }
                    },
                    Err(LevelDBErr::Sqlite(rusqlite::Error::QueryReturnedNoRows)) => {
                        //there is no palette saved, so we create one using only our current map.
                        //This happens when a new world is created
                        let mut saved_to_loaded = SavedToLoadedBlockIdMap::default();
                        let mut loaded_to_saved = LoadedToSavedBlockIdMap::default();
                        let palette = create_palette_from_block_id_map(resources.registry.as_ref(), &mut saved_to_loaded, &mut loaded_to_saved);
                        if let Some (err) = db.immediate_execute_command(|sql| sql.execute(INSERT_WORLD_INFO, rusqlite::params!["block_palette", palette])) {
                            error!("Error creating block palette! {:?}", err);
                            return;
                        }
                        commands.insert_resource(saved_to_loaded);
                        commands.insert_resource(loaded_to_saved);
                    }
                    Err(e) => {
                        error!("Error messing with block palette in db: {:?}", e);
                        return;
                    },
                };
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

fn create_block_id_maps_from_palette(data: &Vec<u8>, registry: &BlockRegistry) -> Option<(SavedToLoadedBlockIdMap, LoadedToSavedBlockIdMap)> {
    match bincode::deserialize::<BlockNameIdMap>(data) {
        Ok(saved_map) => {
            let mut saved_to_loaded = SavedToLoadedBlockIdMap::default();
            let mut loaded_to_saved = LoadedToSavedBlockIdMap::default();
            for (name, saved_map_id) in saved_map.iter() {
                match registry.id_map.get(name) {
                    Some(loaded_map_id) => 
                    {
                        info!("Mapped saved block {:?} (id: {:?}) to loaded block id {:?}", name, saved_map_id, loaded_map_id);
                        saved_to_loaded.insert(*saved_map_id, *loaded_map_id);
                        loaded_to_saved.insert(*loaded_map_id, *saved_map_id);
                    },
                    None => {
                        error!("Unknown block name in palette: {:?}", name);
                        return None
                    },
                }
            }
            Some((saved_to_loaded, loaded_to_saved))
        },
        Err(e) => {
            error!("couldn't load block id map from palette, {}", e);
            None
        },
    }
}

//if we have loaded blocks that aren't in the world, this will add them to the map.
//returns the new palette map to be saved to disk
fn create_palette_from_block_id_map(registry: &BlockRegistry, saved_to_loaded: &mut SavedToLoadedBlockIdMap, loaded_to_saved: &mut LoadedToSavedBlockIdMap) -> Vec<u8> {
    let mut palette = BlockNameIdMap::new();
    for (name, id) in registry.id_map.iter() {
        if !loaded_to_saved.map.contains_key(id) {
            //this block was not mapped to a saved block id, so its a new block. We set it to itself.
            //ids are always in the range 0..<block_count, and we verify that we have all saved blocks loaded before this point
            //so id must be >= saved_block_count, therefore, we aren't overwriting anything with this save_to_loaded.insert
            let new_id = id.with_id(saved_to_loaded.max_key_id+1);
            assert_eq!(saved_to_loaded.insert(new_id, *id), None);
            loaded_to_saved.insert(*id,new_id);
            info!("Added block {:?} to block palette with id {:?}", name, id);
        }
        palette.insert(name.clone(), *loaded_to_saved.get(id).unwrap());
    }
    bincode::serialize(&palette).unwrap()
}