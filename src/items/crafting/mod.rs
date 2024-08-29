use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::util::iterators::Volume;

use super::ItemSystemSet;

pub mod recipes;

pub struct CraftingPlugin;

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(recipes::RecipesPlugin)
            .configure_sets(
                Update,
                (
                    CraftingSystemSet::RecipeCheckers.in_set(ItemSystemSet::UsageProcessing),
                    CraftingSystemSet::RecipePicker.in_set(ItemSystemSet::UsageProcessing),
                    CraftingSystemSet::RecipeActor.in_set(ItemSystemSet::UsageProcessing),
                )
                    .chain(),
            )
            .add_systems(
                Update,
                recipe_picker.in_set(CraftingSystemSet::RecipePicker),
            )
            .add_event::<RecipeCandidateEvent>()
            .add_event::<RecipeCraftedEvent>()
            .register_type::<CraftingHammer>();
    }
}

//recipe ids may not be stable across program runs. to get a specific id for a recipe,
// use recipe registry
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub usize);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum CraftingSystemSet {
    RecipeCheckers, //take in item usage events, fires recipe candidate event
    RecipePicker,   //take in recipe candidate events, fires recipe crafted event
    RecipeActor,    //take in recipe crafted events, does some action in the world
}

#[derive(Event, Clone, Copy)]
pub struct RecipeCandidateEvent(RecipeCraftedEvent);

#[derive(Event, Clone, Copy)]
pub struct RecipeCraftedEvent {
    pub volume: Volume,
    pub id: RecipeId,
}

#[derive(
    Copy, Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component)]
pub struct CraftingHammer;

fn recipe_picker(
    mut candidate_events: EventReader<RecipeCandidateEvent>,
    mut crafted_writer: EventWriter<RecipeCraftedEvent>,
    mut candidates: Local<Vec<RecipeCraftedEvent>>,
) {
    for RecipeCandidateEvent(event) in candidate_events.read() {
        let mut discarded = false;
        //to remove ambiguities, if two bounding boxes of recipes overlap, only keep the larger volume of the two
        //O(n^2) alogrithm, but shouldn't be very many recipes per update
        candidates.retain(|other| {
            if event.volume.intersects(other.volume) {
                //keep larger of the two
                let keep_other = other.volume.volume() > event.volume.volume();
                discarded = discarded || keep_other;
                return keep_other;
            }
            //doesn't intersect, so keep both
            true
        });
        if !discarded {
            candidates.push(*event);
        }
    }
    crafted_writer.send_batch(candidates.drain(..));
}
