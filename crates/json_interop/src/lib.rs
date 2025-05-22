use bevy::{ecs::world::World, prelude::*};
use serde::Deserialize;
use std::sync::Arc;

pub struct JsonInteropPlugin;

impl Plugin for JsonInteropPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EffectEvent>()
            .init_resource::<ConditionRegistry>();
    }
}

#[derive(Event, Debug)]
pub struct EffectEvent {
    pub effect: Effect,
    pub primary_entity: Option<Entity>,
    pub secondary_entity: Option<Entity>,
}

// Effect types
// these will get cloned a lot
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
/// to handle effect, add event reader for `DialogueEffect` in relevant crate
pub enum Effect {
    ChangeFriendship(i64),
    OpenUI(Arc<str>),
    TriggerEvent(Arc<str>),
    StartQuest(Arc<str>),
    GiveItem { name: Arc<str>, quantity: u32 },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Condition {
    Not(Box<Condition>),
    HasItem(Arc<str>),
    Time(Arc<str>),
    MinHearts(u32),
}

#[derive(Clone, Copy)]
pub struct ConditionEvaluation<'a> {
    pub condition: &'a Condition,
    pub primary_entity: Option<Entity>,
    pub secondary_entity: Option<Entity>,
}

impl<'a> ConditionEvaluation<'a> {
    pub fn new(
        condition: &'a Condition,
        primary_entity: Option<Entity>,
        secondary_entity: Option<Entity>,
    ) -> Self {
        Self {
            condition,
            primary_entity,
            secondary_entity,
        }
    }
}

#[derive(Resource, Default)]
pub struct ConditionRegistry {
    map: Vec<Box<dyn Fn(&World, ConditionEvaluation) -> Option<bool> + Send + Sync>>,
}

impl ConditionRegistry {
    fn insert(
        &mut self,
        function: Box<dyn Fn(&World, ConditionEvaluation) -> Option<bool> + Send + Sync>,
    ) {
        self.map.push(function);
    }

    /// checks if the first matching condition function returns true
    /// if no matches, returns true
    pub fn matches(&self, world: &World, eval: ConditionEvaluation) -> bool {
        self.map
            .iter()
            .filter_map(|cond| cond(world, eval))
            .next()
            // return true if no matches
            .unwrap_or(true)
    }
}

pub trait BuildConditionRegistry {
    fn add_condition<
        Cond: Fn(&World, ConditionEvaluation) -> Option<bool> + Send + Sync + 'static,
    >(
        &mut self,
        function: Cond,
    ) -> &mut Self;
}

impl BuildConditionRegistry for App {
    fn add_condition<
        Cond: Fn(&World, ConditionEvaluation) -> Option<bool> + Send + Sync + 'static,
    >(
        &mut self,
        function: Cond,
    ) -> &mut Self {
        let mut registry = self
            .world_mut()
            .get_resource_or_insert_with(ConditionRegistry::default);
        registry.insert(Box::new(function));
        self
    }
}
