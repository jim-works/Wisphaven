use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::world::*;

use super::ItemSystemSet;

pub mod recipes;

pub struct CraftingPlugin;

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(recipes::RecipesPlugin)
            .configure_sets(
                Update,
                (
                    CraftingSystemSet::RecipeCheckers.in_set(ItemSystemSet::ItemUsageProcessing),
                    CraftingSystemSet::RecipePicker.in_set(ItemSystemSet::ItemUsageProcessing),
                    CraftingSystemSet::RecipeActor.in_set(ItemSystemSet::ItemUsageProcessing),
                )
                    .chain(),
            )
            .add_systems(Update, recipe_picker.in_set(CraftingSystemSet::RecipePicker))
            .add_event::<RecipeCandidateEvent>()
            .add_event::<RecipeCraftedEvent>();
    }
}

//recipe ids may not be stable across program runs. to get a specific id for a recipe,
// use recipe registry
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub usize);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum CraftingSystemSet {
    RecipeCheckers, //take in item usage events, fires recipe candidate event
    RecipePicker,   //take in recipe candidate events, fires recipe crafted event
    RecipeActor,    //take in recipe crafted events, does some action in the world
}

#[derive(Event, Clone, Copy)]
pub struct RecipeCandidateEvent(RecipeCraftedEvent);

#[derive(Event, Clone, Copy)]
pub struct RecipeCraftedEvent {
    pub volume: BlockVolume,
    pub id: RecipeId,
}

fn recipe_picker(
    mut candidate_events: EventReader<RecipeCandidateEvent>,
    mut crafted_writer: EventWriter<RecipeCraftedEvent>,
    mut candidates: Local<Vec<RecipeCraftedEvent>>,
) {
    for RecipeCandidateEvent(event) in candidate_events.iter() {
        let mut contained = false;
        candidates.retain(|other| {
            contained = contained || other.volume.contains(event.volume);
            !event.volume.contains(other.volume)
        });
        if !contained {
            candidates.push(*event);
        }
    }
    crafted_writer.send_batch(candidates.drain(..));
}