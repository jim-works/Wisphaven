use bevy::{prelude::*, asset::LoadState};

use crate::{mesher::TerrainTexture, world::BlockResources, items::ItemResources};

use super::ItemTextureMap;

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameLoadState {
    #[default]
    LoadingAssets,
    Done
}

#[derive(Resource, Default)]
pub struct TexturesLoaded(pub bool);

pub struct SerializationStatePlugin;

impl Plugin for SerializationStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameLoadState>()
            .insert_resource(TexturesLoaded::default())
            .add_systems(Update, (check_load_state, check_textures).run_if(in_state(GameLoadState::LoadingAssets)))
        ;
    }
}

pub fn check_textures(
    mut progress: ResMut<TexturesLoaded>,
    block_textures: Res<TerrainTexture>,
    item_textures: Res<ItemTextureMap>,
    assets: Res<AssetServer>
) {
    if !progress.0 
        && block_textures.0.iter().all(|x| assets.get_load_state(x) == LoadState::Loaded)
        &&  item_textures.0.values().all(|x| assets.get_load_state(x) == LoadState::Loaded) {
        progress.0 = true;
        info!("Finished loading textures")
    }
}

pub fn check_load_state(
    mut next: ResMut<NextState<GameLoadState>>,
    block_types: Option<Res<BlockResources>>,
    item_types: Option<Res<ItemResources>>,
    block_textures: Res<TexturesLoaded>
) {
    if block_textures.0 && block_types.is_some() && item_types.is_some() {
        info!("Finished loading!");
        next.set(GameLoadState::Done)
    }
}