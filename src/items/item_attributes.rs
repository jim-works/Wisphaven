use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{inventory::Inventory, DropItemEvent, ItemSystemSet, UnequipItemEvent, UseItemEvent};

pub struct ItemAttributesPlugin;

impl Plugin for ItemAttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, consume_items.in_set(ItemSystemSet::DropPickup))
            .register_type::<ConsumableItem>()
            .register_type::<ItemSwingSpeed>()
            .register_type::<ItemUseSpeed>();
    }
}

//item that gets consumed on use
#[derive(Clone, Hash, Eq, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ConsumableItem;

#[derive(Clone, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ItemSwingSpeed {
    pub windup: Duration,
    pub backswing: Duration,
}

#[derive(Clone, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ItemUseSpeed {
    pub windup: Duration,
    pub backswing: Duration,
}

fn consume_items(
    mut drop_writer: EventWriter<DropItemEvent>,
    mut unequip_writer: EventWriter<UnequipItemEvent>,
    mut events: EventReader<UseItemEvent>,
    consumable_query: Query<&ConsumableItem>,
    mut inventory_query: Query<&mut Inventory>,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf: _,
    } in events.read()
    {
        if !consumable_query.contains(stack.id) {
            continue;
        }
        if let Some(slot_num) = inventory_slot {
            if let Ok(mut inv) = inventory_query.get_mut(*user) {
                inv.drop_items(*slot_num, 1, &mut drop_writer, &mut unequip_writer);
            }
        }
    }
}
