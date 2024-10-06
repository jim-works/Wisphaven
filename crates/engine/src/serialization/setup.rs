use bevy::asset::LoadedFolder;
pub use bevy::prelude::*;
use bevy::utils::HashMap;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::items::crafting::recipes::{BasicBlockRecipe, NamedBasicBlockRecipe, RecipeList};

use crate::items::{
    ItemIcon, ItemId, ItemName, ItemNameIdMap, ItemRegistry, ItemResources, NamedItemIcon,
};
use crate::mesher::item_mesher::GenerateItemMeshEvent;
use crate::mesher::{mesh_single_block, TerrainTexture};
use crate::serialization::db::{LevelDB, LevelDBErr};
use crate::serialization::queries::{
    CREATE_CHUNK_TABLE, CREATE_WORLD_INFO_TABLE, INSERT_WORLD_INFO, LOAD_WORLD_INFO,
};
use crate::serialization::{LoadingBlocks, LoadingItems, LoadingRecipes};
use crate::util::string::Version;
use crate::world::settings::GraphicsSettings;
use crate::world::{events::CreateLevelEvent, settings::Settings, Level};
use crate::world::{
    BlockId, BlockName, BlockNameIdMap, BlockRegistry, BlockResources, Id, LevelData,
    LevelLoadState, NamedBlockMesh,
};

use super::{state, BlockTextureMap, ItemTextureMap, LoadedToSavedIdMap, SavedToLoadedIdMap};

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(load_settings())
            .insert_resource(load_graphics_settings())
            //instantiate entities that we need to load
            .add_systems(PreStartup, load_folders)
            //initiate loading of each type of scene
            .add_systems(
                Update,
                (
                    load_block_textures.run_if(resource_exists::<LoadingBlockTextures>),
                    load_item_textures.run_if(resource_exists::<LoadingItemTextures>),
                    (|| (LoadingBlocks, "blocks"))
                        .pipe(start_loading_scene::<LoadingBlockScenes>)
                        .run_if(resource_exists::<LoadingBlockScenes>),
                    (|| (LoadingRecipes, "recipes"))
                        .pipe(start_loading_scene::<LoadingRecipeScenes>)
                        .run_if(resource_exists::<LoadingRecipeScenes>),
                    (|| (LoadingItems, "items"))
                        .pipe(start_loading_scene::<LoadingItemScenes>)
                        .run_if(resource_exists::<LoadingItemScenes>),
                    (|mut n: ResMut<NextState<state::GameLoadState>>| {
                        info!("finished preloading, loading assets now!");
                        n.set(state::GameLoadState::LoadingAssets)
                    })
                    .run_if(not(resource_exists::<LoadingBlockTextures>))
                    .run_if(not(resource_exists::<LoadingItemTextures>))
                    .run_if(not(resource_exists::<LoadingBlockScenes>))
                    .run_if(not(resource_exists::<LoadingRecipeScenes>))
                    .run_if(not(resource_exists::<LoadingItemScenes>)),
                )
                    .run_if(in_state(state::GameLoadState::Preloading)),
            )
            //create registries/recipe lists
            .add_systems(
                Update,
                (
                    load_block_registry,
                    load_item_registry,
                    load_recipe_list.run_if(resource_exists::<BlockResources>),
                )
                    .run_if(in_state(state::GameLoadState::LoadingAssets)),
            )
            //create level
            .add_systems(
                Update,
                on_level_created.run_if(
                    in_state(state::GameLoadState::Done)
                        .and_then(in_state(LevelLoadState::NotLoaded)),
                ),
            );
    }
}

#[derive(Resource, Deref, Clone)]
pub struct LoadingBlockTextures(Handle<LoadedFolder>);

#[derive(Resource, Deref, Clone)]
pub struct LoadingItemTextures(Handle<LoadedFolder>);

#[derive(Resource, Deref, Clone)]
pub struct LoadingBlockScenes(Handle<LoadedFolder>);

#[derive(Resource, Deref, Clone)]
pub struct LoadingItemScenes(Handle<LoadedFolder>);

#[derive(Resource, Deref, Clone)]
pub struct LoadingRecipeScenes(Handle<LoadedFolder>);

pub fn load_settings() -> Settings {
    Settings::default()
}

pub fn load_graphics_settings() -> GraphicsSettings {
    GraphicsSettings::default()
}

//begins loading the terrain texture images and creates the filename->texture id map
pub fn load_folders(mut commands: Commands, assets: Res<AssetServer>, settings: Res<Settings>) {
    commands.insert_resource(LoadingBlockTextures(
        assets.load_folder(settings.block_tex_path),
    ));
    commands.insert_resource(LoadingItemTextures(
        assets.load_folder(settings.item_tex_path),
    ));
    commands.insert_resource(LoadingBlockScenes(
        assets.load_folder(settings.block_type_path),
    ));
    commands.insert_resource(LoadingItemScenes(
        assets.load_folder(settings.item_type_path),
    ));
    commands.insert_resource(LoadingRecipeScenes(
        assets.load_folder(settings.recipe_path),
    ));
}

pub fn load_block_textures(
    mut commands: Commands,
    assets: Res<Assets<LoadedFolder>>,
    settings: Res<Settings>,
    loading_blocks: Res<LoadingBlockTextures>,
    mut asset_events: EventReader<AssetEvent<LoadedFolder>>,
) {
    for event in asset_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            if *id == loading_blocks.0.id() {
                commands.remove_resource::<LoadingBlockTextures>();
                let textures: Vec<Handle<Image>> = assets
                    .get(loading_blocks.0.id())
                    .unwrap()
                    .handles
                    .iter()
                    .map(|handle| handle.clone().typed())
                    .collect();
                if textures.is_empty() {
                    warn!("No block textures found at {}", settings.block_tex_path);
                    return;
                }

                let mut names = HashMap::new();
                for (i, texture) in textures.iter().enumerate() {
                    //`get_handle_path` returns "textures/blocks/folder/name.png"
                    //this removes the "textures/blocks" to leave us with "folder/name.png"
                    let texture_name: PathBuf = texture
                        .path()
                        .unwrap()
                        .path()
                        .strip_prefix(settings.block_tex_path)
                        .unwrap()
                        .to_path_buf();
                    info!(
                        "Loaded block texture name {} with id {}",
                        texture_name.display(),
                        i
                    );
                    names.insert(texture_name, i as u32);
                }
                commands.insert_resource(BlockTextureMap(names));
                commands.insert_resource(TerrainTexture(textures));
            }
        }
    }
}

//begins loading the item texture images and creates the filename->texture id map
pub fn load_item_textures(
    mut commands: Commands,
    assets: Res<Assets<LoadedFolder>>,
    settings: Res<Settings>,
    loading_blocks: Res<LoadingItemTextures>,
    mut asset_events: EventReader<AssetEvent<LoadedFolder>>,
) {
    for event in asset_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            if *id == loading_blocks.0.id() {
                commands.remove_resource::<LoadingItemTextures>();
                let textures: Vec<Handle<Image>> = assets
                    .get(loading_blocks.0.id())
                    .unwrap()
                    .handles
                    .iter()
                    .map(|handle| handle.clone().typed())
                    .collect();
                if textures.is_empty() {
                    warn!("No item textures found at {}", settings.item_tex_path);
                    return;
                }

                let mut names = HashMap::new();
                for (i, texture) in textures.iter().enumerate() {
                    //`get_handle_path` returns "textures/items/folder/name.png"
                    //this removes the "textures/items" to leave us with "folder/name.png"
                    let texture_name: PathBuf = texture
                        .path()
                        .unwrap()
                        .path()
                        .strip_prefix(settings.item_tex_path)
                        .unwrap()
                        .to_path_buf();
                    info!(
                        "Loaded item texture name {} with id {}",
                        texture_name.display(),
                        i
                    );
                    names.insert(texture_name, texture.clone());
                }
                commands.insert_resource(ItemTextureMap(names));
            }
        }
    }
}

pub fn start_loading_scene<Scene: Resource + std::ops::Deref<Target = Handle<LoadedFolder>>>(
    input: In<(impl Bundle + Clone, &'static str)>,
    mut commands: Commands,
    assets: Res<Assets<LoadedFolder>>,
    settings: Res<Settings>,
    loading_scenes: Res<Scene>,
    mut asset_events: EventReader<AssetEvent<LoadedFolder>>,
) {
    let (bundle, name) = input.0;
    for event in asset_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            if *id == loading_scenes.id() {
                commands.remove_resource::<Scene>();
                let scenes: Vec<Handle<DynamicScene>> = assets
                    .get(&loading_scenes.deref().clone())
                    .unwrap()
                    .handles
                    .iter()
                    .map(|handle| handle.clone().typed())
                    .collect();
                if scenes.is_empty() {
                    warn!("No {} found at {}", name, settings.block_type_path);
                    return;
                }

                info!("Spawning {} {} scenes", scenes.len(), name);
                for scene in scenes {
                    commands.spawn((
                        DynamicSceneBundle { scene, ..default() },
                        Name::new(name),
                        bundle.clone(),
                    ));
                }
            }
        }
    }
}

pub fn load_block_registry(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    texture_map: Res<BlockTextureMap>,
    loading_blocks: Query<(Entity, Option<&Children>), With<LoadingBlocks>>,
    block_name_query: Query<&BlockName>,
    name_resolution_query: Query<&NamedBlockMesh>,
    block_resources: Option<Res<BlockResources>>,
) {
    //make sure there are no still loading block scenes before we make the registry
    if block_resources.is_some()
        || loading_blocks
            .iter()
            .any(|(_, opt_children)| opt_children.is_none())
    {
        return;
    }
    let mut registry = BlockRegistry::default();
    registry
        .id_map
        .insert(BlockName::core("empty"), BlockId(Id::Empty));
    for (scene_entity, children) in loading_blocks.iter() {
        info!("Loading block scene");
        commands.entity(scene_entity).remove::<LoadingBlocks>();
        for child in children.unwrap() {
            //do name resolution
            let mut single_mesh = None;
            if let Ok(named_mesh) = name_resolution_query.get(*child) {
                let mut mesh = named_mesh.clone().into_block_mesh(&texture_map);
                mesh.single_mesh = mesh_single_block(&mesh, &mut meshes);
                single_mesh = mesh.single_mesh.clone();
                commands
                    .entity(*child)
                    .insert(mesh)
                    .remove::<NamedBlockMesh>();
            }
            match block_name_query.get(*child) {
                Ok(name) => registry.add_basic(name.clone(), single_mesh, *child, &mut commands),
                Err(e) => warn!("Block doesn't have a name! Error {:?}", e),
            }
        }
    }

    info!("Finished loading {} block types", registry.id_map.len());
    commands.insert_resource(BlockResources {
        registry: std::sync::Arc::new(registry),
    });
}

pub fn load_item_registry(
    mut commands: Commands,
    mut generate_item_mesh: EventWriter<GenerateItemMeshEvent>,
    texture_map: Res<ItemTextureMap>,
    loading_items: Query<(Entity, Option<&Children>), With<LoadingItems>>,
    item_name_query: Query<&ItemName>,
    name_resolution_query: Query<&NamedItemIcon>,
    item_resources: Option<Res<ItemResources>>,
) {
    //make sure there are no still loading block scenes before we make the registry
    if item_resources.is_some()
        || loading_items
            .iter()
            .any(|(_, opt_children)| opt_children.is_none())
    {
        return;
    }
    let mut registry = ItemRegistry::default();
    for (scene_entity, children) in loading_items.iter() {
        info!("Loading item scene");
        commands.entity(scene_entity).remove::<LoadingItems>();
        for child in children.unwrap() {
            //do name resolution
            if let Ok(named_icon) = name_resolution_query.get(*child) {
                match texture_map.0.get(&named_icon.path) {
                    Some(handle) => {
                        commands
                            .entity(*child)
                            .insert(ItemIcon(handle.clone()))
                            .remove::<NamedItemIcon>();
                        generate_item_mesh.send(GenerateItemMeshEvent(*child));
                    }
                    None => {
                        error!(
                            "Icon not found in item texture map: {}",
                            named_icon.path.display()
                        )
                    }
                }
            }
            match item_name_query.get(*child) {
                Ok(name) => registry.add_basic(name.clone(), *child, &mut commands),
                Err(e) => warn!("Item doesn't have a name! Error {:?}", e),
            }
        }
    }

    info!("Finished loading {} item types", registry.id_map.len());
    commands.insert_resource(ItemResources {
        registry: std::sync::Arc::new(registry),
    });
}

pub fn load_recipe_list(
    mut commands: Commands,
    loading_recipes: Query<(Entity, Option<&Children>), With<LoadingRecipes>>,
    named_recipe: Query<&NamedBasicBlockRecipe>,
    block_registry: Res<BlockResources>,
    recipe_list: Option<Res<RecipeList>>,
) {
    //make sure there are no still loading recipe scenes before we make the registry
    if recipe_list.is_some()
        || loading_recipes
            .iter()
            .any(|(_, opt_children)| opt_children.is_none())
    {
        return;
    }
    //not happy with this, along with all the register type nonsense.
    //todo - look into asset v2 when updating to 0.12 or just use serde_json
    //  don't think I'll add a lot of components to recipes after all...
    let recipes: Vec<BasicBlockRecipe> = loading_recipes
        .iter()
        .flat_map(|(scene_entity, opt_children)| {
            commands.entity(scene_entity).remove::<LoadingRecipes>();
            let mut recipes = Vec::with_capacity(opt_children.unwrap().len());
            for recipe_entity in opt_children.unwrap() {
                let NamedBasicBlockRecipe { recipe, products } =
                    named_recipe.get(*recipe_entity).unwrap();
                let mut id_recipe = Vec::with_capacity(recipe.len());
                for y_row in recipe.iter() {
                    let mut y_row_id = Vec::with_capacity(y_row.len());
                    for x_row in y_row.iter() {
                        y_row_id.push(
                            x_row
                                .iter()
                                .map(|opt_name| {
                                    opt_name
                                        .as_ref()
                                        .and_then(|name| block_registry.registry.get_basic(name))
                                })
                                .collect::<Vec<_>>(),
                        )
                    }
                    id_recipe.push(y_row_id);
                }
                let id_products = products
                    .iter()
                    .map(|(coord, name)| (*coord, block_registry.registry.get_basic(name).into()))
                    .collect();
                commands
                    .entity(*recipe_entity)
                    .remove::<NamedBasicBlockRecipe>();
                recipes.push(BasicBlockRecipe::new(&id_recipe, id_products).unwrap());
            }
            recipes
        })
        .collect();

    info!("Finished loading {} recipes", recipes.len());
    commands.insert_resource(RecipeList::new(recipes));
}

pub fn on_level_created(
    mut reader: EventReader<CreateLevelEvent>,
    settings: Res<Settings>,
    block_resources: Res<BlockResources>,
    item_resources: Res<ItemResources>,
    mut next_state: ResMut<NextState<LevelLoadState>>,
    mut commands: Commands,
) {
    const MAX_DBS: u32 = 1;
    if let Some(event) = reader.read().next() {
        info!("on level created event received");
        fs::create_dir_all(settings.env_path).unwrap();
        let db = LevelDB::new(
            std::path::Path::new(settings.env_path)
                .join(event.name.to_owned() + ".db")
                .as_path(),
        );
        match db {
            Ok(mut db) => {
                if let Some(err) =
                    db.execute_command_sync(|sql| sql.execute(CREATE_CHUNK_TABLE, []))
                {
                    error!("Error creating chunk table: {:?}", err);
                    return;
                }
                if let Some(err) =
                    db.execute_command_sync(|sql| sql.execute(CREATE_WORLD_INFO_TABLE, []))
                {
                    error!("Error creating world info table: {:?}", err);
                    return;
                }
                match check_level_version(&mut db) {
                    Err(err) => {
                        error!("Error checking level version: {:?}", err);
                        return;
                    }
                    _ => {}
                }
                load_block_palette(&mut db, &mut commands, &block_resources.registry);
                load_item_palette(&mut db, &mut commands, &item_resources.registry);
                commands.insert_resource(db);
                commands.insert_resource(Level(Arc::new(LevelData::new(event.name, 8008135))));
                next_state.set(LevelLoadState::Loading);
                info!("in state loading!");
            }
            Err(e) => {
                error!("couldn't open db {}", e);
            }
        }
    }
}
fn check_level_version(db: &mut LevelDB) -> Result<(), LevelDBErr> {
    match db.execute_query_sync(LOAD_WORLD_INFO, rusqlite::params!["version"], |row| {
        row.get::<_, Vec<u8>>(0)
    }) {
        Ok(data) => match bincode::deserialize::<&str>(&data) {
            Ok(version) => {
                let my_version = Version::game_version();
                let saved_version = Version::from(version);
                info!(
                    "saved version of level is {:?}, my version is {:?}",
                    saved_version, my_version
                );
                if saved_version > my_version && !my_version.game_compatible(&saved_version) {
                    error!(
                        "Opening a newer world version than I can handle: {:?}",
                        version
                    );
                    return Err(LevelDBErr::NewWorldVersion);
                }
            }
            Err(e) => {
                error!("Corrupt world version: {:?}", e);
                return Err(LevelDBErr::Bincode(e));
            }
        },
        Err(LevelDBErr::Sqlite(rusqlite::Error::QueryReturnedNoRows)) => {} //this is fine - new worlds have no version
        Err(e) => {
            error!("Error getting world version from db: {:?}", e);
            return Err(e);
        }
    }
    if let Some(err) = db.execute_command_sync(|sql| {
        sql.execute(
            INSERT_WORLD_INFO,
            rusqlite::params![
                "version",
                bincode::serialize(env!("CARGO_PKG_VERSION")).unwrap()
            ],
        )
    }) {
        return Err(err);
    }
    Ok(())
}
fn load_block_palette(db: &mut LevelDB, commands: &mut Commands, registry: &BlockRegistry) {
    match db.execute_query_sync(LOAD_WORLD_INFO, rusqlite::params!["block_palette"], |row| {
        row.get(0)
    }) {
        Ok(data) => {
            match create_block_id_maps_from_palette(&data, registry) {
                Some((mut saved_to_loaded, mut loaded_to_saved)) => {
                    //if we have new blocks that were not in the palette before, add them
                    let palette = create_palette_from_block_id_map(
                        registry,
                        &mut saved_to_loaded,
                        &mut loaded_to_saved,
                    );
                    if let Some(err) = db.execute_command_sync(|sql| {
                        sql.execute(
                            INSERT_WORLD_INFO,
                            rusqlite::params!["block_palette", palette],
                        )
                    }) {
                        error!("Error updating block palette! {:?}", err);
                        return;
                    }
                    //put the updated maps in the world
                    commands.insert_resource(saved_to_loaded);
                    commands.insert_resource(loaded_to_saved);
                }
                None => {
                    error!("Couldn't create saved block id map!");
                }
            }
        }
        Err(LevelDBErr::Sqlite(rusqlite::Error::QueryReturnedNoRows)) => {
            //there is no palette saved, so we create one using only our current map.
            //This happens when a new world is created
            let mut saved_to_loaded = SavedToLoadedIdMap::default();
            let mut loaded_to_saved = LoadedToSavedIdMap::default();
            let palette = create_palette_from_block_id_map(
                registry,
                &mut saved_to_loaded,
                &mut loaded_to_saved,
            );
            if let Some(err) = db.execute_command_sync(|sql| {
                sql.execute(
                    INSERT_WORLD_INFO,
                    rusqlite::params!["block_palette", palette],
                )
            }) {
                error!("Error creating block palette! {:?}", err);
                return;
            }
            commands.insert_resource(saved_to_loaded);
            commands.insert_resource(loaded_to_saved);
        }
        Err(e) => {
            error!("Error messing with block palette in db: {:?}", e);
        }
    }
}

#[allow(clippy::ptr_arg)] //must be vec, sql cannot retrieve [u8]
fn create_block_id_maps_from_palette(
    data: &Vec<u8>,
    registry: &BlockRegistry,
) -> Option<(SavedToLoadedIdMap<BlockId>, LoadedToSavedIdMap<BlockId>)> {
    match bincode::deserialize::<BlockNameIdMap>(data) {
        Ok(saved_map) => {
            let mut saved_to_loaded = SavedToLoadedIdMap::default();
            let mut loaded_to_saved = LoadedToSavedIdMap::default();
            for (name, saved_map_id) in saved_map.iter() {
                match registry.id_map.get(name) {
                    Some(loaded_map_id) => {
                        info!(
                            "Mapped saved block {:?} (id: {:?}) to loaded block id {:?}",
                            name, saved_map_id, loaded_map_id
                        );
                        saved_to_loaded.insert(*saved_map_id, *loaded_map_id);
                        loaded_to_saved.insert(*loaded_map_id, *saved_map_id);
                    }
                    None => {
                        error!("Unknown block name in palette: {:?}", name);
                        return None;
                    }
                }
            }
            Some((saved_to_loaded, loaded_to_saved))
        }
        Err(e) => {
            error!("couldn't load block id map from palette, {}", e);
            None
        }
    }
}

//if we have loaded blocks that aren't in the world, this will add them to the map.
//returns the new palette map to be saved to disk
fn create_palette_from_block_id_map(
    registry: &BlockRegistry,
    saved_to_loaded: &mut SavedToLoadedIdMap<BlockId>,
    loaded_to_saved: &mut LoadedToSavedIdMap<BlockId>,
) -> Vec<u8> {
    let mut palette = BlockNameIdMap::new();
    for (name, id) in registry.id_map.iter() {
        if !loaded_to_saved.map.contains_key(id) {
            //this block was not mapped to a saved block id, so its a new block. We set it to itself.
            //ids are always in the range 0..<block_count, and we verify that we have all saved blocks loaded before this point
            //so id must be >= saved_block_count, therefore, we aren't overwriting anything with this save_to_loaded.insert
            let new_id = BlockId(id.0.with_id(saved_to_loaded.max_key_id + 1));
            assert_eq!(saved_to_loaded.insert(new_id, *id), None);
            loaded_to_saved.insert(*id, new_id);
            info!("Added block {:?} to block palette with id {:?}", name, id);
        }
        palette.insert(name.clone(), loaded_to_saved.get(id).unwrap());
    }
    bincode::serialize(&palette).unwrap()
}

fn load_item_palette(db: &mut LevelDB, commands: &mut Commands, registry: &ItemRegistry) {
    match db.execute_query_sync(LOAD_WORLD_INFO, rusqlite::params!["item_palette"], |row| {
        row.get(0)
    }) {
        Ok(data) => {
            match create_item_id_maps_from_palette(&data, registry) {
                Some((mut saved_to_loaded, mut loaded_to_saved)) => {
                    //if we have new blocks that were not in the palette before, add them
                    let palette = create_palette_from_item_id_map(
                        registry,
                        &mut saved_to_loaded,
                        &mut loaded_to_saved,
                    );
                    if let Some(err) = db.execute_command_sync(|sql| {
                        sql.execute(
                            INSERT_WORLD_INFO,
                            rusqlite::params!["item_palette", palette],
                        )
                    }) {
                        error!("Error updating item palette! {:?}", err);
                        return;
                    }
                    //put the updated maps in the world
                    commands.insert_resource(saved_to_loaded);
                    commands.insert_resource(loaded_to_saved);
                }
                None => {
                    error!("Couldn't create saved block id map!");
                }
            }
        }
        Err(LevelDBErr::Sqlite(rusqlite::Error::QueryReturnedNoRows)) => {
            //there is no palette saved, so we create one using only our current map.
            //This happens when a new world is created
            let mut saved_to_loaded = SavedToLoadedIdMap::default();
            let mut loaded_to_saved = LoadedToSavedIdMap::default();
            let palette = create_palette_from_item_id_map(
                registry,
                &mut saved_to_loaded,
                &mut loaded_to_saved,
            );
            if let Some(err) = db.execute_command_sync(|sql| {
                sql.execute(
                    INSERT_WORLD_INFO,
                    rusqlite::params!["item_palette", palette],
                )
            }) {
                error!("Error creating item palette! {:?}", err);
                return;
            }
            commands.insert_resource(saved_to_loaded);
            commands.insert_resource(loaded_to_saved);
        }
        Err(e) => {
            error!("Error messing with item palette in db: {:?}", e);
        }
    }
}

#[allow(clippy::ptr_arg)] //must be vec, sql cannot retrieve [u8]
fn create_item_id_maps_from_palette(
    data: &Vec<u8>,
    registry: &ItemRegistry,
) -> Option<(SavedToLoadedIdMap<ItemId>, LoadedToSavedIdMap<ItemId>)> {
    match bincode::deserialize::<ItemNameIdMap>(data) {
        Ok(saved_map) => {
            let mut saved_to_loaded = SavedToLoadedIdMap::default();
            let mut loaded_to_saved = LoadedToSavedIdMap::default();
            for (name, saved_map_id) in saved_map.iter() {
                match registry.id_map.get(name) {
                    Some(loaded_map_id) => {
                        info!(
                            "Mapped saved item {:?} (id: {:?}) to loaded item id {:?}",
                            name, saved_map_id, loaded_map_id
                        );
                        saved_to_loaded.insert(*saved_map_id, *loaded_map_id);
                        loaded_to_saved.insert(*loaded_map_id, *saved_map_id);
                    }
                    None => {
                        error!("Unknown item name in palette: {:?}", name);
                        return None;
                    }
                }
            }
            Some((saved_to_loaded, loaded_to_saved))
        }
        Err(e) => {
            error!("couldn't load item id map from palette, {}", e);
            None
        }
    }
}

//if we have loaded items that aren't in the world, this will add them to the map.
//returns the new palette map to be saved to disk
fn create_palette_from_item_id_map(
    registry: &ItemRegistry,
    saved_to_loaded: &mut SavedToLoadedIdMap<ItemId>,
    loaded_to_saved: &mut LoadedToSavedIdMap<ItemId>,
) -> Vec<u8> {
    let mut palette = ItemNameIdMap::new();
    for (name, id) in registry.id_map.iter() {
        if !loaded_to_saved.map.contains_key(id) {
            //this item was not mapped to a saved item id, so its a new item. We set it to itself.
            //ids are always in the range 0..<count, and we verify that we have all saved items loaded before this point
            //so id must be >= saved_count, therefore, we aren't overwriting anything with this save_to_loaded.insert
            let new_id = ItemId(id.0.with_id(saved_to_loaded.max_key_id + 1));
            assert_eq!(saved_to_loaded.insert(new_id, *id), None);
            loaded_to_saved.insert(*id, new_id);
            info!("Added item {:?} to item palette with id {:?}", name, id);
        }
        palette.insert(name.clone(), loaded_to_saved.get(id).unwrap());
    }
    bincode::serialize(&palette).unwrap()
}
