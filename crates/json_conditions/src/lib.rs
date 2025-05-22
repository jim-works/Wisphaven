use bevy::prelude::*;
use engine::items::{ItemId, ItemName, ItemResources, inventory::Inventory};
use interfaces::components::Id;
use json_interop::{Condition, *};

pub struct ConditionsPlugin;

impl Plugin for ConditionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_condition(route);
    }
}

fn route(world: &World, eval: ConditionEvaluation) -> Option<bool> {
    match eval.condition {
        Condition::Not(condition) => not(condition.as_ref(), world, eval),
        Condition::HasItem(name) => has_item(name.as_ref(), world, eval),
        Condition::Time(_) => todo!(),
        Condition::MinHearts(_) => todo!(),
    }
}

fn not(condition: &Condition, world: &World, eval: ConditionEvaluation) -> Option<bool> {
    let Some(registry) = world.get_resource::<ConditionRegistry>() else {
        return None;
    };
    return Some(!registry.matches(world, ConditionEvaluation { condition, ..eval }));
}

fn has_item(item_name_str: &str, world: &World, eval: ConditionEvaluation) -> Option<bool> {
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
