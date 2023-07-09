use bevy::{prelude::*, asset::LoadState};

use crate::mesher::TerrainTexture;

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameLoadState {
    #[default]
    LoadingAssets,
    Done
}

#[derive(Resource, Default)]
pub struct BlockTypesLoaded(pub bool);

#[derive(Resource, Default)]
pub struct BlockTexturesLoaded(pub bool);

pub struct SerializationStatePlugin;

impl Plugin for SerializationStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameLoadState>()
            .insert_resource(BlockTexturesLoaded::default())
            .insert_resource(BlockTypesLoaded::default())
            .add_systems((check_load_state, check_textures).in_set(OnUpdate(GameLoadState::LoadingAssets)))
        ;
    }
}

pub fn check_textures(
    mut progress: ResMut<BlockTexturesLoaded>,
    textures: Res<TerrainTexture>,
    assets: Res<AssetServer>
) {
    if !progress.0 && textures.0.iter().all(|x| assets.get_load_state(x) == LoadState::Loaded) {
        progress.0 = true;
        info!("Finished loading textures")
    }
}

pub fn check_load_state(
    mut next: ResMut<NextState<GameLoadState>>,
    block_types: Res<BlockTypesLoaded>,
    block_textures: Res<BlockTexturesLoaded>
) {
    if block_textures.0 && block_types.0 {
        info!("Finished loading!");
        next.set(GameLoadState::Done)
    }
}