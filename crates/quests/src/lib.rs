use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, LoadedFolder, io::Reader},
    prelude::*,
    utils::HashMap,
};
use engine::items::{ItemName, ItemRegistry, ItemResources, ItemStack};
use interfaces::scheduling::LevelSystemSet;
use json_interop::Effect;
use serde::Deserialize;
use thiserror::Error;

pub struct QuestsPlugin;

impl Plugin for QuestsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Quests>()
            .init_asset::<QuestAsset>()
            .init_asset_loader::<QuestAssetLoader>()
            .add_systems(Startup, trigger_loading)
            .add_systems(FixedUpdate, cache_on_load.in_set(LevelSystemSet::Tick));
    }
}

#[derive(Component)]
pub struct Quest(pub Handle<QuestAsset>);

#[derive(Component)]
pub struct ActiveQuest;

#[derive(Component)]
pub struct CompletedQuest;

fn trigger_loading(asset_server: Res<AssetServer>, mut quests: ResMut<Quests>) {
    quests.loading_folder = asset_server.load_folder("quests");
}

fn cache_on_load(
    mut quests: ResMut<Quests>,
    mut assets: ResMut<Assets<QuestAsset>>,
    folder_assets: Res<Assets<LoadedFolder>>,
    items: Res<ItemResources>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if quests.folder_loaded {
        return;
    }
    info!("checking if folder deps loaded...");
    match asset_server.dependency_load_state(&quests.loading_folder) {
        bevy::asset::DependencyLoadState::NotLoaded | bevy::asset::DependencyLoadState::Loading => {
            return;
        }
        bevy::asset::DependencyLoadState::Loaded => (),
        bevy::asset::DependencyLoadState::Failed(asset_load_error) => {
            error!("error loading quest: {:?}", asset_load_error)
        }
    }
    info!("quest folder loaded");
    let Some(folder) = folder_assets.get(&quests.loading_folder) else {
        error!(
            "somehow quest folder is not loaded yet even though we check the dependency load state."
        );
        return;
    };
    quests.folder_loaded = true;
    for handle in folder
        .handles
        .iter()
        .filter_map(|handle| handle.clone().try_typed::<QuestAsset>().ok())
    {
        let Some(quest) = assets.get_mut(&handle) else {
            error!("somehow quest asset not ready even though folder is loaded xd");
            continue;
        };
        quest.cache(&items.registry, &mut commands);

        let key = format!("{}.{}", quest.character, quest.name).leak();
        info!("Quest loaded: {} - {:?}", key, quest);
        quests
            .quests
            .insert(key, commands.spawn(Quest(handle)).id());
    }
}

#[derive(Resource, Default)]
pub struct Quests {
    /// key is `character.quest` ex `paul.findMyWife`
    pub quests: HashMap<&'static str, Entity>,
    loading_folder: Handle<LoadedFolder>,
    folder_loaded: bool,
}

#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct QuestAsset {
    pub character: Arc<str>,
    pub name: Arc<str>,
    pub title: Arc<str>,
    pub description: Arc<str>,
    pub rewards: Vec<QuestReward>,
}

impl QuestAsset {
    fn cache(&mut self, registry: &ItemRegistry, commands: &mut Commands) {
        for reward in self.rewards.iter_mut() {
            reward.cache(registry, commands)
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct QuestReward {
    #[serde(default)]
    pub description: Option<Arc<str>>,
    pub reward: Effect,
    #[serde(skip)]
    pub cached_item: Option<ItemStack>,
}

impl QuestReward {
    pub fn cache(&mut self, registry: &ItemRegistry, commands: &mut Commands) {
        if let Effect::GiveItem { name, quantity } = self.reward.clone() {
            if let Ok(item_name) = ItemName::try_from(name.as_ref()) {
                self.cached_item = registry
                    .get_entity(registry.get_id(&item_name), commands)
                    .map(|item| ItemStack::new(item, quantity));
            }
        }
    }
}

#[derive(Default)]
struct QuestAssetLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum QuestAssetLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    /// A [JSON Error](serde_json::error::Error)
    #[error("Could not parse the JSON: {0}")]
    JsonError(#[from] serde_json::error::Error),
}

impl AssetLoader for QuestAssetLoader {
    type Asset = QuestAsset;
    type Settings = ();
    type Error = QuestAssetLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        match serde_json::from_slice::<QuestAsset>(&bytes) {
            Ok(asset) => Ok(asset),
            Err(e) => {
                error!("error loading quest: {:?}", e);
                Err(QuestAssetLoaderError::JsonError(e))
            }
        }
    }

    fn extensions(&self) -> &[&str] {
        &["quest"]
    }
}
