use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
    utils::HashMap,
};
use interfaces::scheduling::LevelSystemSet;
use serde::Deserialize;
use thiserror::Error;

pub struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Dialog>()
            .init_asset_loader::<DialogAssetLoader>()
            .init_resource::<Dialogs>()
            .add_event::<AdvaceDialog>()
            .add_systems(Startup, setup)
            .add_systems(
                FixedUpdate,
                (print_on_load, init_active_dialog, advance_dialog)
                    .chain()
                    .in_set(LevelSystemSet::Tick),
            );
    }
}

#[derive(Resource, Default)]
pub struct Dialogs {
    /// key is `character.event` ex `paul.introduction`
    pub dialogs: HashMap<&'static str, Handle<Dialog>>,
    loading_dialogs: Vec<Handle<Dialog>>,
}

impl Dialogs {
    pub fn load_dialog(&mut self, path: &str, asset_server: &AssetServer) {
        self.loading_dialogs.push(asset_server.load(path));
    }
}

fn setup(asset_server: Res<AssetServer>, mut dialogs: ResMut<Dialogs>) {
    dialogs.load_dialog("dialog/test.json", &asset_server);
}

fn advance_dialog(
    mut commands: Commands,
    mut reader: EventReader<AdvaceDialog>,
    mut query: Query<&mut ActiveDialog>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    for AdvaceDialog {
        dialog_entity,
        selected_response,
    } in reader.read()
    {
        let Ok(mut active_dialog) = query.get_mut(*dialog_entity) else {
            error!(
                "trying to advance dialog for invalid entity {:?}",
                dialog_entity
            );
            continue;
        };
        let Some(dialog) = dialog_assets.get(&active_dialog.handle) else {
            error!(
                "trying to advance dialog for invalid dialog handle {:?}. removing component",
                active_dialog.handle
            );
            commands.entity(*dialog_entity).remove::<ActiveDialog>();
            continue;
        };
        // default active node to root if not set
        let active_node = active_dialog
            .active_node
            .clone()
            .unwrap_or(dialog.node.clone());
        let new_active_node = match active_node.as_ref() {
            DialogNode::Decision { choices, .. } => {
                // todo - more involved. for now we always pick the first
                choices.get(0).map(|choice| choice.node.clone())
            }
            DialogNode::Message {
                effects: _, node, ..
            } => {
                // todo - more involved. apply effects
                node.clone()
            }
            DialogNode::Response { options, .. } => {
                let i = match selected_response {
                    Some(i) => *i,
                    None => {
                        warn!(
                            "no response number sent for advancing response node. defaulting to 0"
                        );
                        0
                    }
                };
                // todo - more involved. apply effects
                options.get(i).and_then(|option| option.node.clone())
            }
            DialogNode::Jump { jump_to, .. } => dialog.id_map.get(&jump_to.clone()).cloned(),
        };
        active_dialog.active_node = new_active_node;
        if active_dialog.active_node.is_none() {
            //dialog ended, remove component
            commands.entity(*dialog_entity).remove::<ActiveDialog>();
        }
    }
}

fn init_active_dialog(
    mut query: Query<&mut ActiveDialog, Added<ActiveDialog>>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    for mut active in query.iter_mut() {
        if active.active_node.is_none() {
            active.active_node = dialog_assets
                .get(&active.handle)
                .map(|dialog| dialog.node.clone());
        }
    }
}

fn print_on_load(
    mut dialogs: ResMut<Dialogs>,
    dialog_assets: Res<Assets<Dialog>>,
    mut buffer: Local<Vec<Handle<Dialog>>>,
    mut completed: Local<Vec<(&'static str, Handle<Dialog>)>>,
) {
    buffer.clear();
    completed.clear();
    for dialog_handle in dialogs.loading_dialogs.drain(..) {
        let Some(loaded_dialog) = dialog_assets.get(&dialog_handle) else {
            buffer.push(dialog_handle);
            continue;
        };

        let key = format!("{}.{}", loaded_dialog.character, loaded_dialog.event).leak();
        info!("Dialog loaded: {} - {:?}", key, loaded_dialog);
        completed.push((key, dialog_handle));
    }
    dialogs.loading_dialogs.append(&mut buffer);
    for (key, handle) in completed.drain(..) {
        dialogs.dialogs.insert(key, handle);
    }
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
        match serde_json::from_slice::<Dialog>(&bytes) {
            Ok(mut asset) => {
                asset.populate_ids();
                info!("this is the id map: {:?}", asset.id_map);
                Ok(asset)
            }
            Err(e) => {
                error!("error loading dialog: {:?}", e);
                Err(DialogAssetLoaderError::JsonError(e))
            }
        }
    }
}

#[derive(Component)]
pub struct ActiveDialog {
    pub handle: Handle<Dialog>,
    pub active_node: Option<Arc<DialogNode>>,
}

impl ActiveDialog {
    pub fn new(handle: Handle<Dialog>) -> Self {
        Self {
            handle,
            active_node: None,
        }
    }
}

#[derive(Event)]
pub struct AdvaceDialog {
    pub dialog_entity: Entity,
    pub selected_response: Option<usize>,
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
    pub node: Option<Arc<DialogNode>>,
}

#[derive(Debug, Deserialize)]
pub struct DialogChoice {
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub node: Arc<DialogNode>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DialogNode {
    #[serde(rename = "decision")]
    Decision {
        #[serde(default)]
        id: Option<Arc<str>>,
        choices: Vec<DialogChoice>,
    },

    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        id: Option<Arc<str>>,
        message: String,
        #[serde(default)]
        effects: Vec<Effect>,
        #[serde(default)]
        node: Option<Arc<DialogNode>>,
    },

    #[serde(rename = "response")]
    Response {
        #[serde(default)]
        id: Option<Arc<str>>,
        options: Vec<ResponseOption>,
    },

    #[serde(rename = "jump")]
    Jump {
        #[serde(rename = "jumpTo")]
        jump_to: Arc<str>,
        #[serde(default)]
        id: Option<Arc<str>>,
        // if i ever decide to add a Arc<DialogNode> in here, make sure it's weak to avoid cycles. (or not maybe in the future idc)
    },
}

impl DialogNode {
    pub fn iter(self: Arc<DialogNode>) -> DialogNodeIter {
        let mut stack = Vec::new();
        stack.push(self.clone());
        DialogNodeIter { stack }
    }

    pub fn id(&self) -> Option<Arc<str>> {
        match self {
            DialogNode::Decision { id, .. } => id.clone(),
            DialogNode::Message { id, .. } => id.clone(),
            DialogNode::Response { id, .. } => id.clone(),
            DialogNode::Jump { id, .. } => id.clone(),
        }
    }
}

pub struct DialogNodeIter {
    stack: Vec<Arc<DialogNode>>,
}

impl Iterator for DialogNodeIter {
    type Item = Arc<DialogNode>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        // Push children onto the stack for later traversal
        match node.as_ref() {
            DialogNode::Decision { choices, .. } => {
                for choice in choices.iter().rev() {
                    self.stack.push(choice.node.clone());
                }
            }
            DialogNode::Message { node, .. } => {
                if let Some(next_node) = node {
                    self.stack.push(next_node.clone());
                }
            }
            DialogNode::Response { options, .. } => {
                for option in options.iter().rev() {
                    if let Some(next_node) = &option.node {
                        self.stack.push(next_node.clone());
                    }
                }
            }
            DialogNode::Jump { .. } => {
                // Jump nodes don't have children directly attached
            }
        }

        Some(node)
    }
}

#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct Dialog {
    pub character: String,
    pub event: String,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub node: Arc<DialogNode>,
    #[serde(skip)]
    id_map: HashMap<Arc<str>, Arc<DialogNode>>,
}

impl Dialog {
    pub fn iter(&self) -> DialogNodeIter {
        self.node.clone().iter()
    }
    pub fn populate_ids(&mut self) {
        self.id_map.clear();
        for node in self.iter() {
            if let Some(id) = node.id() {
                self.id_map.insert(id, node.clone());
            }
        }
    }
}
