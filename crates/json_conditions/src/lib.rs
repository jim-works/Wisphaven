use bevy::{ecs::world::DeferredWorld, prelude::*};
use engine::items::{ItemId, ItemName, ItemResources, inventory::Inventory};
use interfaces::components::Id;
use json_interop::{Condition, *};

pub struct ConditionsPlugin;

impl Plugin for ConditionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_condition(has_item);
    }
}

fn has_item(world: &DeferredWorld, eval: ConditionEvaluation) -> Option<bool> {
    info!("in has_item");
    let Condition::HasItem(item_name_str) = eval.condition else {
        // expect item name in string format
        return None;
    };
    info!("checking has_item for {}", item_name_str);
    let Ok(item_name) = ItemName::try_from(item_name_str.as_ref()) else {
        error!(
            "Can't parse item name in has_item condition {}",
            item_name_str
        );
        return Some(false);
    };
    let item_resources = world.get_resource::<ItemResources>()?;
    let id = item_resources.registry.get_id(&item_name);
    if matches!(id.0, Id::Empty) {
        error!("Missing item id in has_item condition {}", item_name);
        return Some(false);
    }
    let mut entities = [eval.primary_entity, eval.secondary_entity]
        .into_iter()
        .filter_map(|e| e);
    Some(entities.any(|entity| {
        let Ok(Some(inventory)) = world.get_entity(entity).map(|e| e.get::<Inventory>()) else {
            error!(
                "cannot get inventory for {:?} in has_item condition",
                entity
            );
            return false;
        };
        inventory.iter().any(|stack_opt| {
            stack_opt
                .and_then(|stack| {
                    world
                        .get_entity(stack.id)
                        .ok()
                        .and_then(|item_entity| item_entity.get::<ItemId>())
                        .map(|item_id| id == *item_id)
                })
                .unwrap_or(false)
        })
    }))
}
