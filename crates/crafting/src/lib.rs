use bevy::prelude::*;
use engine::{
    items::{inventory::Inventory, ItemName, ItemResources, ItemStack, MaxStackSize},
    world::LevelSystemSet,
};
use serde::{Deserialize, Serialize};

pub struct RecipePlugin;

impl Plugin for RecipePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Recipe>()
            .add_event::<CraftEvent>()
            .add_systems(FixedUpdate, craft.in_set(LevelSystemSet::Tick))
            .add_systems(
                FixedPreUpdate,
                cache_recipe_entities.run_if(resource_exists::<ItemResources>),
            );
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
    // optimize - probably fine to own this, but maybe can improve using entity as id and mapping in multiplayer?
    pub recipe: CachedEntityRecipe,
}

#[derive(Component, Debug, Clone)]
pub struct CachedEntityRecipe {
    pub inputs: Vec<ItemStack>,
    pub output: ItemStack,
}

impl CachedEntityRecipe {
    //returns None if an item was not found
    pub fn from_recipe(recipe: &Recipe, items: &Res<ItemResources>) -> Option<Self> {
        let Some(output_entity) = items.registry.get_basic(&recipe.output.0) else {
            warn!("recipe has unknown output item name {:?}", recipe.output.0);
            return None;
        };
        let inputs = recipe
            .inputs
            .iter()
            .filter_map(|(item, count)| {
                let Some(item_entity) = items.registry.get_basic(item) else {
                    warn!("recipe has unknown item name {:?}", item);
                    return None;
                };
                Some(ItemStack::new(item_entity, *count))
            })
            .collect::<Vec<_>>();
        if inputs.len() != recipe.inputs.len() {
            None
        } else {
            Some(Self {
                inputs,
                output: ItemStack::new(output_entity, recipe.output.1),
            })
        }
    }

    pub fn has_any_input(&self, inventory: &Inventory) -> bool {
        self.inputs
            .iter()
            .any(|stack| inventory.number_of_type(stack.id) >= stack.size)
    }

    pub fn has_inputs(&self, inventory: &Inventory) -> bool {
        self.inputs
            .iter()
            .all(|stack| inventory.number_of_type(stack.id) >= stack.size)
    }

    // accounts for having recipe inputs and having the output slot available
    pub fn can_craft(&self, inventory: &Inventory, pickup_query: &Query<&MaxStackSize>) -> bool {
        inventory.can_pickup_item(self.output, pickup_query) && self.has_inputs(inventory)
    }
}

fn craft(
    mut reader: EventReader<CraftEvent>,
    mut inv_query: Query<&mut Inventory>,
    pickup_query: Query<&MaxStackSize>,
) {
    for CraftEvent { crafter, recipe } in reader.read() {
        info!("recv craft event");
        let Ok(mut inventory) = inv_query.get_mut(*crafter) else {
            warn!("trying to craft from something without an inventory");
            continue;
        };
        // make sure we have space for the output before removing items
        if recipe.can_craft(&inventory, &pickup_query) {
            inventory.pickup_item(recipe.output, &pickup_query);

            // remove inputs from inventory
            for input in recipe.inputs.iter() {
                inventory.remove_items(*input);
            }

            info!("crafted item");
        } else {
            info!("couldn't craft item");
        }
    }
}

fn cache_recipe_entities(
    query: Query<(Entity, &Recipe), Without<CachedEntityRecipe>>,
    mut commands: Commands,
    items: Res<ItemResources>,
) {
    for (entity, recipe) in query.iter() {
        let Some(mut ec) = commands.get_entity(entity) else {
            warn!("adding cached recipe to invalid entity somehow");
            continue;
        };
        match CachedEntityRecipe::from_recipe(recipe, &items) {
            Some(cached) => {
                ec.insert(cached);
                info!("cached recipe {:?}", recipe);
            }
            None => {
                warn!("failed to lookup items for recipe {:?}", recipe)
            }
        }
    }
}
