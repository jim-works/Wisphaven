use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use serde::Deserialize;
use thiserror::Error;

pub struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Dialog>()
            .init_asset_loader::<DialogAssetLoader>()
            .init_resource::<LoadedDialog>()
            .add_systems(Startup, setup)
            .add_systems(Update, print_on_load);
    }
}

#[derive(Resource, Default)]
struct LoadedDialog(Handle<Dialog>, bool);

fn setup(asset_server: Res<AssetServer>, mut loaded_dialog: ResMut<LoadedDialog>) {
    loaded_dialog.0 = asset_server.load("dialog/test.json");
}

fn print_on_load(mut loaded_dialog: ResMut<LoadedDialog>, dialog_assets: Res<Assets<Dialog>>) {
    let dialog_instance = dialog_assets.get(&loaded_dialog.0);

    // Can't print results if the assets aren't ready
    if loaded_dialog.1 {
        return;
    }

    if dialog_instance.is_none() {
        info!("Custom Asset Not Ready");
        return;
    }

    info!("Custom asset loaded: {:?}", dialog_instance.unwrap());

    // Once printed, we won't print again
    loaded_dialog.1 = true;
}

#[derive(Default)]
struct DialogAssetLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum DialogAssetLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    /// A [JSON Error](serde_json::error::Error)
    #[error("Could not parse the JSON: {0}")]
    JsonError(#[from] serde_json::error::Error),
}

impl AssetLoader for DialogAssetLoader {
    type Asset = Dialog;
    type Settings = ();
    type Error = DialogAssetLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        match serde_json::from_slice(&bytes) {
            Ok(asset) => Ok(asset),
            Err(e) => {
                error!("error loading dialog: {:?}", e);
                Err(DialogAssetLoaderError::JsonError(e))
            }
        }
    }
}

// Enum to represent the different possible value types in conditions
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ConditionValue {
    String(String),
    Int(i32),
    Float(f32),
}

// Effect types
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Effect {
    ChangeFriendship {
        #[serde(rename = "changeFriendship")]
        change_friendship: i64,
    },
    TriggerEvent {
        #[serde(rename = "triggerEvent")]
        trigger_event: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Time {
        time: String,
    },
    MinHearts {
        #[serde(rename = "minHearts")]
        min_hearts: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct ResponseOption {
    pub message: String,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub node: Option<Box<DialogNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DialogNode {
    #[serde(rename = "decision")]
    Decision {
        #[serde(default)]
        id: Option<String>,
        choices: Vec<DialogChoice>,
    },

    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        id: Option<String>,
        message: String,
        #[serde(default)]
        effects: Vec<Effect>,
        #[serde(default)]
        node: Option<Box<DialogNode>>,
    },

    #[serde(rename = "response")]
    Response {
        #[serde(default)]
        id: Option<String>,
        options: Vec<ResponseOption>,
    },

    #[serde(rename = "jump")]
    Jump {
        #[serde(rename = "jumpTo")]
        jump_to: String,
        #[serde(default)]
        id: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct DialogChoice {
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub node: DialogNode,
}

#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct Dialog {
    pub character: String,
    pub event: String,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub node: DialogNode,
}
