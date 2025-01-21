use bevy::prelude::*;
use engine::{
    actors::LocalPlayer,
    items::{inventory::Inventory, ItemName, ItemResources, ItemStack, MaxStackSize},
    world::LevelSystemSet,
};
use serde::{Deserialize, Serialize};

pub struct RecipePlugin;

impl Plugin for RecipePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Recipe>()
            .add_event::<CraftEvent>()
            .add_systems(FixedUpdate, (craft).in_set(LevelSystemSet::Tick))
            .add_systems(Update, test_crafting.in_set(LevelSystemSet::Main));
    }
}

#[derive(
    Default, Clone, Debug, PartialEq, Eq, Hash, Component, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, FromWorld)]
pub struct Recipe {
    pub inputs: Vec<(ItemName, u32)>,
    pub output: (ItemName, u32),
}

#[derive(Event)]
pub struct CraftEvent {
    pub crafter: Entity,
    // probably inefficient to own this, but unsure of how it works with networking
    // think it is fine since each recipe will not have many items
    pub recipe: Recipe,
}

fn test_crafting(
    craft_button: Res<ButtonInput<KeyCode>>,
    mut writer: EventWriter<CraftEvent>,
    player_query: Query<Entity, With<LocalPlayer>>,
    recipes: Query<&Recipe>,
) {
    let Ok(player) = player_query.get_single() else {
        return;
    };
    if craft_button.just_pressed(KeyCode::KeyT) {
        for recipe in recipes.iter() {
            info!("crafting! {:?}", recipe);
            writer.send(CraftEvent {
                crafter: player,
                recipe: recipe.clone(),
            });
        }
    }
}

fn craft(
    mut reader: EventReader<CraftEvent>,
    mut inv_query: Query<&mut Inventory>,
    items: Res<ItemResources>,
    pickup_query: Query<&MaxStackSize>,
) {
    for CraftEvent { crafter, recipe } in reader.read() {
        let Ok(mut inventory) = inv_query.get_mut(*crafter) else {
            warn!("trying to craft from something without an inventory");
            continue;
        };
        // check we have all inputs in inventory
        let has_inputs = recipe.inputs.iter().all(|(item, count)| {
            let Some(item_entity) = items.registry.get_basic(item) else {
                warn!("recipe has unknown item name {:?}", item);
                return false;
            };
            info!(
                "counted type: {:?} for entity {:?}",
                inventory.number_of_type(item_entity),
                item_entity
            );
            inventory.number_of_type(item_entity) >= *count
        });

        if !has_inputs {
            info!("missing inputs");
            continue;
        }

        let Some(output_entity) = items.registry.get_basic(&recipe.output.0) else {
            warn!("recipe has unknown output item name {:?}", recipe.output.0);
            continue;
        };
        let output_stack = ItemStack::new(output_entity, recipe.output.1);
        // make sure we have space for the output before removing items
        if inventory.can_pickup_item(output_stack, &pickup_query) {
            inventory.pickup_item(output_stack, &pickup_query);

            // remove inputs from inventory
            for (item, count) in recipe.inputs.iter() {
                let Some(item_entity) = items.registry.get_basic(item) else {
                    warn!("recipe has unknown item name (somehow got removed between checking inputs and crafting, oh well) {:?}", item);
                    continue;
                };
                inventory.remove_items(ItemStack::new(item_entity, *count));
            }

            info!("crafted item");
        }
    }
}
