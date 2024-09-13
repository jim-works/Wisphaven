use bevy::{asset::LoadState, prelude::*};

use crate::{
    items::{crafting::recipes::RecipeList, ItemResources},
    mesher::TerrainTexture,
    world::{atmosphere::SkyboxCubemap, BlockResources},
    GameState,
};

use super::ItemTextureMap;

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameLoadState {
    #[default]
    Preloading,
    LoadingAssets,
    CreatingLevel,
    Done,
}

#[derive(Resource, Default)]
pub struct TexturesLoaded(pub bool);

pub struct SerializationStatePlugin;

impl Plugin for SerializationStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameLoadState>()
            .insert_resource(TexturesLoaded::default())
            .add_systems(
                Update,
                (check_load_state, check_textures).run_if(
                    in_state(GameLoadState::LoadingAssets).and_then(in_state(GameState::Game)),
                ),
            );
    }
}

pub fn check_textures(
    mut progress: ResMut<TexturesLoaded>,
    block_textures: Res<TerrainTexture>,
    item_textures: Res<ItemTextureMap>,
    assets: Res<AssetServer>,
) {
    if !progress.0
        && block_textures
            .0
            .iter()
            .all(|x| assets.get_load_state(x) == Some(LoadState::Loaded))
        && item_textures
            .0
            .values()
            .all(|x| assets.get_load_state(x) == Some(LoadState::Loaded))
    {
        progress.0 = true;
        info!("Finished loading textures")
    }
}

pub fn check_load_state(
    mut next: ResMut<NextState<GameLoadState>>,
    block_types: Option<Res<BlockResources>>,
    item_types: Option<Res<ItemResources>>,
    recipes: Option<Res<RecipeList>>,
    block_textures: Res<TexturesLoaded>,
    skybox: Option<Res<SkyboxCubemap>>,
) {
    if block_textures.0
        && block_types.is_some()
        && item_types.is_some()
        && recipes.is_some()
        && skybox.is_some()
    {
        info!("Finished loading!");
        next.set(GameLoadState::CreatingLevel)
    }
}
