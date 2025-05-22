use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
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
    quests
        .loading_quests
        .push(asset_server.load("quests/test.json"));
}

fn cache_on_load(
    mut quests: ResMut<Quests>,
    mut assets: ResMut<Assets<QuestAsset>>,
    mut buffer: Local<Vec<Handle<QuestAsset>>>,
    mut completed: Local<Vec<(&'static str, Entity)>>,
    items: Res<ItemResources>,
    mut commands: Commands,
) {
    buffer.clear();
    completed.clear();
    for handle in quests.loading_quests.drain(..) {
        let Some(quest) = assets.get_mut(&handle) else {
            buffer.push(handle);
            continue;
        };
        quest.cache(&items.registry, &mut commands);

        let key = format!("{}.{}", quest.character, quest.name).leak();
        info!("Quest loaded: {} - {:?}", key, quest);
        completed.push((key, commands.spawn(Quest(handle)).id()));
    }
    quests.loading_quests.append(&mut buffer);
    for (key, handle) in completed.drain(..) {
        quests.quests.insert(key, handle);
    }
}

#[derive(Resource, Default)]
pub struct Quests {
    /// key is `character.quest` ex `paul.findMyWife`
    pub quests: HashMap<&'static str, Entity>,
    loading_quests: Vec<Handle<QuestAsset>>,
}

#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct QuestAsset {
    pub character: Arc<str>,
    pub name: Arc<str>,
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
}
