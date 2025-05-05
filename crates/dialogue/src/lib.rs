#![feature(let_chains)]
use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
    utils::HashMap,
};
use interfaces::scheduling::LevelSystemSet;
use serde::Deserialize;
use thiserror::Error;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Dialogue>()
            .init_asset_loader::<DialogueAssetLoader>()
            .init_resource::<Dialogues>()
            .add_event::<AdvanceDialogue>()
            .add_systems(Startup, setup)
            .add_systems(
                FixedUpdate,
                (print_on_load, init_active_dialogue, advance_dialogue)
                    .chain()
                    .in_set(LevelSystemSet::Tick),
            );
    }
}

#[derive(Resource, Default)]
pub struct Dialogues {
    /// key is `character.event` ex `paul.introduction`
    pub dialogues: HashMap<&'static str, Handle<Dialogue>>,
    loading_dialogues: Vec<Handle<Dialogue>>,
}

impl Dialogues {
    pub fn load_dialogue(&mut self, path: &str, asset_server: &AssetServer) {
        self.loading_dialogues.push(asset_server.load(path));
    }
}

fn setup(asset_server: Res<AssetServer>, mut dialogues: ResMut<Dialogues>) {
    dialogues.load_dialogue("dialogue/test.json", &asset_server);
}

fn advance_dialogue(
    mut commands: Commands,
    mut reader: EventReader<AdvanceDialogue>,
    mut query: Query<&mut ActiveDialogue>,
    dialogue_assets: Res<Assets<Dialogue>>,
) {
    for AdvanceDialogue {
        dialogue_entity,
        selected_response,
    } in reader.read()
    {
        info!("advancing dialogue");
        let Ok(mut active_dialogue) = query.get_mut(*dialogue_entity) else {
            error!(
                "trying to advance dialogue for invalid entity {:?}",
                dialogue_entity
            );
            continue;
        };
        let Some(dialogue) = dialogue_assets.get(&active_dialogue.handle) else {
            error!(
                "trying to advance dialogue for invalid dialogue handle {:?}. removing component",
                active_dialogue.handle
            );
            commands.entity(*dialogue_entity).remove::<ActiveDialogue>();
            continue;
        };
        // continue advancing until we hit a user interaction
        loop {
            // default active node to root if not set
            let active_node_opt = active_dialogue
                .active_node
                .clone()
                .or(if active_dialogue.init {
                    None
                } else {
                    Some(dialogue.node.clone())
                });
            let Some(active_node) = active_node_opt else {
                //dialogue ended, remove component
                info!("dialogue ended, removing component");
                commands.entity(*dialogue_entity).remove::<ActiveDialogue>();
                break;
            };
            let new_active_node = match active_node.as_ref() {
                DialogueNode::Decision { choices, .. } => {
                    // todo - more involved. for now we always pick the first
                    choices.get(0).map(|choice| choice.node.clone())
                }
                DialogueNode::Message {
                    effects: _, node, ..
                } => {
                    // todo - more involved. apply effects
                    node.clone()
                }
                DialogueNode::Response { options, .. } => {
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
                DialogueNode::Jump { jump_to, .. } => {
                    dialogue.id_map.get(&jump_to.clone()).cloned()
                }
            };
            active_dialogue.active_node = new_active_node;
            match &active_dialogue.active_node {
                Some(node) => {
                    if node.is_visual_node() {
                        // wait for user interaction
                        break;
                    }
                }
                None => {
                    // conversation ended, wait until we advance again to remove component
                    info!("conversation ended");
                    break;
                }
            };
        }
    }
}

/// must be ordered before advance_dialogue
fn init_active_dialogue(
    mut query: Query<(Entity, &mut ActiveDialogue), Added<ActiveDialogue>>,
    dialogue_assets: Res<Assets<Dialogue>>,
    mut advance_writer: EventWriter<AdvanceDialogue>,
) {
    for (dialogue_entity, mut active) in query.iter_mut() {
        if active.active_node.is_none() {
            active.active_node = dialogue_assets
                .get(&active.handle)
                .map(|dialogue| dialogue.node.clone());
            if let Some(node) = &active.active_node
                && !node.is_visual_node()
            {
                advance_writer.send(AdvanceDialogue {
                    dialogue_entity,
                    selected_response: None,
                });
            }
        }
    }
}

fn print_on_load(
    mut dialogues: ResMut<Dialogues>,
    dialogue_assets: Res<Assets<Dialogue>>,
    mut buffer: Local<Vec<Handle<Dialogue>>>,
    mut completed: Local<Vec<(&'static str, Handle<Dialogue>)>>,
) {
    buffer.clear();
    completed.clear();
    for dialogue_handle in dialogues.loading_dialogues.drain(..) {
        let Some(loaded_dialogue) = dialogue_assets.get(&dialogue_handle) else {
            buffer.push(dialogue_handle);
            continue;
        };

        let key = format!("{}.{}", loaded_dialogue.character, loaded_dialogue.event).leak();
        info!("Dialogue loaded: {} - {:?}", key, loaded_dialogue);
        completed.push((key, dialogue_handle));
    }
    dialogues.loading_dialogues.append(&mut buffer);
    for (key, handle) in completed.drain(..) {
        dialogues.dialogues.insert(key, handle);
    }
}

#[derive(Default)]
struct DialogueAssetLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum DialogueAssetLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    /// A [JSON Error](serde_json::error::Error)
    #[error("Could not parse the JSON: {0}")]
    JsonError(#[from] serde_json::error::Error),
}

impl AssetLoader for DialogueAssetLoader {
    type Asset = Dialogue;
    type Settings = ();
    type Error = DialogueAssetLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        match serde_json::from_slice::<Dialogue>(&bytes) {
            Ok(mut asset) => {
                asset.populate_ids();
                info!("this is the id map: {:?}", asset.id_map);
                Ok(asset)
            }
            Err(e) => {
                error!("error loading dialogue: {:?}", e);
                Err(DialogueAssetLoaderError::JsonError(e))
            }
        }
    }
}

#[derive(Component)]
pub struct ActiveDialogue {
    pub handle: Handle<Dialogue>,
    pub active_node: Option<Arc<DialogueNode>>,
    init: bool,
}

impl ActiveDialogue {
    pub fn new(handle: Handle<Dialogue>) -> Self {
        Self {
            handle,
            active_node: None,
            init: false,
        }
    }
}

#[derive(Event)]
pub struct AdvanceDialogue {
    pub dialogue_entity: Entity,
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
    pub node: Option<Arc<DialogueNode>>,
}

#[derive(Debug, Deserialize)]
pub struct DialogueChoice {
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub node: Arc<DialogueNode>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DialogueNode {
    #[serde(rename = "decision")]
    Decision {
        #[serde(default)]
        id: Option<Arc<str>>,
        choices: Vec<DialogueChoice>,
    },

    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        id: Option<Arc<str>>,
        message: String,
        #[serde(default)]
        effects: Vec<Effect>,
        #[serde(default)]
        node: Option<Arc<DialogueNode>>,
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
        // if i ever decide to add a Arc<DialogueNode> in here, make sure it's weak to avoid cycles. (or not maybe in the future idc)
    },
}

impl DialogueNode {
    pub fn iter(self: Arc<DialogueNode>) -> DialogueNodeIter {
        let mut stack = Vec::new();
        stack.push(self.clone());
        DialogueNodeIter { stack }
    }

    pub fn id(&self) -> Option<Arc<str>> {
        match self {
            DialogueNode::Decision { id, .. } => id.clone(),
            DialogueNode::Message { id, .. } => id.clone(),
            DialogueNode::Response { id, .. } => id.clone(),
            DialogueNode::Jump { id, .. } => id.clone(),
        }
    }

    /// Should the node be displayed to the user?
    pub fn is_visual_node(&self) -> bool {
        match self {
            DialogueNode::Message { .. } => true,
            DialogueNode::Response { .. } => true,
            DialogueNode::Decision { .. } => false,
            DialogueNode::Jump { .. } => false,
        }
    }
}

pub struct DialogueNodeIter {
    stack: Vec<Arc<DialogueNode>>,
}

impl Iterator for DialogueNodeIter {
    type Item = Arc<DialogueNode>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        // Push children onto the stack for later traversal
        match node.as_ref() {
            DialogueNode::Decision { choices, .. } => {
                for choice in choices.iter().rev() {
                    self.stack.push(choice.node.clone());
                }
            }
            DialogueNode::Message { node, .. } => {
                if let Some(next_node) = node {
                    self.stack.push(next_node.clone());
                }
            }
            DialogueNode::Response { options, .. } => {
                for option in options.iter().rev() {
                    if let Some(next_node) = &option.node {
                        self.stack.push(next_node.clone());
                    }
                }
            }
            DialogueNode::Jump { .. } => {
                // Jump nodes don't have children directly attached
            }
        }

        Some(node)
    }
}

#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct Dialogue {
    pub character: String,
    pub event: String,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub node: Arc<DialogueNode>,
    #[serde(skip)]
    id_map: HashMap<Arc<str>, Arc<DialogueNode>>,
}

impl Dialogue {
    pub fn iter(&self) -> DialogueNodeIter {
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
