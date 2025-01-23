use bevy::{asset::LoadState, prelude::*};

use crate::{
    items::ItemResources,
    mesher::TerrainTexture,
    world::{atmosphere::SkyboxCubemap, BlockResources},
    GameState,
};

use super::{ItemTextureMap, RecipesScene};

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameLoadState {
    #[default]
    Preloading,
    LoadingAssets,
    Done,
}

#[derive(Resource, Default)]
pub struct TexturesLoaded(pub bool);

pub struct SerializationStatePlugin;

impl Plugin for SerializationStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameLoadState>()
            .enable_state_scoped_entities::<GameLoadState>()
            .insert_resource(TexturesLoaded::default())
            .add_systems(
                Update,
                (check_load_state, check_textures)
                    .run_if(in_state(GameLoadState::LoadingAssets).and(in_state(GameState::Game))),
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
            .all(|x| matches!(assets.get_load_state(x), Some(LoadState::Loaded)))
        && item_textures
            .0
            .values()
            .all(|x| matches!(assets.get_load_state(x), Some(LoadState::Loaded)))
    {
        progress.0 = true;
        info!("Finished loading textures")
    }
}

pub fn check_load_state(
    mut next: ResMut<NextState<GameLoadState>>,
    block_types: Res<BlockResources>,
    item_types: Res<ItemResources>,
    block_textures: Res<TexturesLoaded>,
    skybox: Option<Res<SkyboxCubemap>>,
    recipes_scene: Query<(), With<RecipesScene>>,
) {
    if block_textures.0
        && block_types.loaded
        && item_types.loaded
        && skybox.is_some()
        && !recipes_scene.is_empty()
    {
        info!("Finished loading!");
        next.set(GameLoadState::Done);
    }
}
