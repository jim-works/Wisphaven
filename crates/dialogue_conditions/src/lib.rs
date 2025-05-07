use bevy::{ecs::world::DeferredWorld, prelude::*};
use dialogue::{BuildDialogueConditionRegistry, Condition};

pub struct DialogueConditionsPlugin;

impl Plugin for DialogueConditionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_dialogue_condition(has_item, "hasItem");
    }
}

/// todo - maybe better types for these functions
fn has_item(world: &DeferredWorld, value: &Condition) -> bool {
    true
}
