#![feature(let_chains)]
use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    ecs::world::DeferredWorld,
    prelude::*,
    utils::HashMap,
};
use interfaces::scheduling::LevelSystemSet;
use json_interop::{Condition, ConditionEvaluation, ConditionRegistry, Effect, EffectEvent};
use serde::Deserialize;
use thiserror::Error;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Dialogue>()
            .init_asset_loader::<DialogueAssetLoader>()
            .init_resource::<Dialogues>()
            .init_resource::<Events<AdvanceDialogue>>()
            .add_event::<EffectEvent>()
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

pub fn advance_dialogue(
    mut commands: Commands,
    mut world: DeferredWorld, // needed for conditions, they can require checking arbitrary data
) {
    // we want to read past events and the ones which just came up, requiring 2 drains
    let mut advance_events = world
        .get_resource_mut::<Events<AdvanceDialogue>>()
        .unwrap()
        .update_drain()
        .collect::<Vec<_>>();

    advance_events.extend(
        world
            .get_resource_mut::<Events<AdvanceDialogue>>()
            .unwrap()
            .update_drain(),
    );

    let mut send_effect_events = Vec::new();

    for AdvanceDialogue {
        dialogue_entity,
        selected_response,
    } in advance_events.drain(..)
    {
        info!("advancing dialogue");
        let mut active_node_opt;
        // I have this weird block for all the read-only world access
        // Ensures they get dropped, so we don't have any borrow issues below when doing the mutable stuff (advancing the actual dialogue)
        {
            let dialogue_assets = world.get_resource::<Assets<Dialogue>>().unwrap();
            let condition_registry = world.get_resource::<ConditionRegistry>().unwrap();
            let Ok(Some(active_dialogue)) = world
                .get_entity(dialogue_entity)
                .map(|e| e.get_components::<&ActiveDialogue>())
            else {
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
                commands.entity(dialogue_entity).remove::<ActiveDialogue>();
                continue;
            };
            // default active node to root if not set
            active_node_opt = active_dialogue
                .active_node
                .clone()
                .or(Some(dialogue.node.clone()));
            // continue advancing until we hit a user interaction
            loop {
                if let Some(active_node) = active_node_opt {
                    // advance node
                    active_node_opt = match active_node.as_ref() {
                        DialogueNode::Decision { choices, .. } => {
                            let first_matching_choice = choices
                                .iter()
                                .filter(|choice| {
                                    choice.conditions.iter().all(|cond| {
                                        condition_registry.matches(
                                            &world,
                                            ConditionEvaluation::new(
                                                cond,
                                                Some(dialogue_entity),
                                                Some(active_dialogue.other),
                                            ),
                                        )
                                    })
                                })
                                .next();
                            match first_matching_choice {
                                Some(choice) => Some(choice.node.clone()),
                                None => {
                                    info!("no matching choice, ending conversation");
                                    None
                                }
                            }
                        }
                        DialogueNode::Message { effects, node, .. } => {
                            send_effects(
                                &mut send_effect_events,
                                effects.iter().cloned(),
                                dialogue_entity,
                                active_dialogue.other,
                            );
                            node.clone()
                        }
                        DialogueNode::Response { options, .. } => {
                            let i = match selected_response {
                                Some(i) => i,
                                None => {
                                    warn!(
                                        "no response number sent for advancing response node. defaulting to 0"
                                    );
                                    0
                                }
                            };
                            let selected_option = options.get(i);
                            if let Some(option) = selected_option {
                                send_effects(
                                    &mut send_effect_events,
                                    option.effects.iter().cloned(),
                                    dialogue_entity,
                                    active_dialogue.other,
                                );
                            }
                            selected_option.and_then(|option| option.node.clone())
                        }
                        DialogueNode::Jump { jump_to, .. } => {
                            dialogue.id_map.get(&jump_to.clone()).cloned()
                        }
                    };
                }
                info!("active node {:?}", active_node_opt);
                info!(
                    "is visual? {}",
                    active_node_opt
                        .as_ref()
                        .map(|n| n.is_visual_node())
                        .unwrap_or(true)
                );
                if active_node_opt
                    .as_ref()
                    .map(|n| n.is_visual_node())
                    .unwrap_or(true)
                {
                    // break out if we need user interaction or if the convo is over
                    break;
                }
            }
            // update stuff
            // I tried doing `drop(active_dialogue)` but did not work for some reason, hence the block
            world.send_event_batch(send_effect_events.drain(..));
            let Ok(mut entity) = world.get_entity_mut(dialogue_entity) else {
                error!("cannot get entity from world on the second try");
                continue;
            };
            match &active_node_opt {
                Some(node) => match entity.get_mut::<ActiveDialogue>() {
                    Some(mut active_dialogue) => {
                        active_dialogue.active_node = Some(node.clone());
                    }
                    None => {
                        error!(
                            "cannot get active dialogue from world on the second try. gonna remove the component anyway for swag."
                        );
                        commands.entity(dialogue_entity).remove::<ActiveDialogue>();
                        continue;
                    }
                },
                None => {
                    info!("dialogue ended, removing component");
                    commands.entity(dialogue_entity).remove::<ActiveDialogue>();
                    break;
                }
            };
        }
    }
}

fn send_effects(
    effect_writer: &mut Vec<EffectEvent>,
    iter: impl Iterator<Item = Effect>,
    dialogue_entity: Entity,
    other_entity: Entity,
) {
    for effect in iter {
        effect_writer.push(EffectEvent {
            effect,
            primary_entity: Some(dialogue_entity),
            secondary_entity: Some(other_entity),
        });
    }
}

/// must be ordered before advance_dialogue
fn init_active_dialogue(
    mut query: Query<(Entity, &mut ActiveDialogue), Added<ActiveDialogue>>,
    dialogue_assets: Res<Assets<Dialogue>>,
    mut advance_writer: EventWriter<AdvanceDialogue>,
) {
    for (dialogue_entity, mut active) in query.iter_mut() {
        info!("init active dialogue system");
        if active.active_node.is_none() {
            active.active_node = dialogue_assets
                .get(&active.handle)
                .map(|dialogue| dialogue.node.clone());
            if let Some(node) = &active.active_node
                && !node.is_visual_node()
            {
                info!("sending advance event to init");
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

#[derive(Component, Debug)]
pub struct ActiveDialogue {
    pub handle: Handle<Dialogue>,
    pub active_node: Option<Arc<DialogueNode>>,
    pub other: Entity,
}

impl ActiveDialogue {
    pub fn new(handle: Handle<Dialogue>, other: Entity) -> Self {
        Self {
            handle,
            other,
            active_node: None,
        }
    }
}

#[derive(Event)]
pub struct AdvanceDialogue {
    pub dialogue_entity: Entity,
    pub selected_response: Option<usize>,
}

// You will see a lot more Arc<str> than String in here
// using ECS requires us to clone a lot, and we never need to modify these strings after loading

#[derive(Debug, Deserialize)]
pub struct ResponseOption {
    pub message: Arc<str>,
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
#[serde(rename_all = "camelCase")]
pub enum DialogueNode {
    #[serde(rename_all = "camelCase")]
    Decision {
        #[serde(default)]
        id: Option<Arc<str>>,
        choices: Vec<DialogueChoice>,
    },

    #[serde(rename_all = "camelCase")]
    Message {
        #[serde(default)]
        id: Option<Arc<str>>,
        message: Arc<str>,
        #[serde(default)]
        effects: Vec<Effect>,
        #[serde(default)]
        node: Option<Arc<DialogueNode>>,
    },

    #[serde(rename_all = "camelCase")]
    Response {
        #[serde(default)]
        id: Option<Arc<str>>,
        options: Vec<ResponseOption>,
    },

    #[serde(rename_all = "camelCase")]
    Jump {
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
    pub character: Arc<str>,
    pub event: Arc<str>,
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
