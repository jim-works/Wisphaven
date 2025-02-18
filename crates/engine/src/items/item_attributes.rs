use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{inventory::Inventory, UseEndEvent};
use interfaces::scheduling::ItemSystemSet;

pub struct ItemAttributesPlugin;

impl Plugin for ItemAttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, consume_items.in_set(ItemSystemSet::DropPickup))
            .register_type::<ConsumeItemOnHit>()
            .register_type::<ConsumeItemOnSucess>()
            .register_type::<ItemSwingSpeed>()
            .register_type::<ItemUseSpeed>();
    }
}

//item that gets consumed on use
#[derive(Clone, Hash, Eq, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component, FromWorld)]
pub struct ConsumeItemOnHit;

//item that gets consumed on use
#[derive(Clone, Hash, Eq, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component, FromWorld)]
pub struct ConsumeItemOnSucess;

#[derive(Clone, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component, FromWorld)]
pub struct ItemSwingSpeed {
    pub windup: Duration,
    pub backswing: Duration,
}

#[derive(Clone, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component, FromWorld)]
pub struct ItemUseSpeed {
    pub windup: Duration,
    pub backswing: Duration,
}

fn consume_items(
    mut events: EventReader<UseEndEvent>,
    on_hit_query: Query<&ConsumeItemOnHit>,
    on_sucess_query: Query<&ConsumeItemOnSucess>,
    mut inventory_query: Query<&mut Inventory>,
) {
    for UseEndEvent {
        user,
        inventory_slot,
        stack,
        result,
    } in events.read()
    {
        if result.is_fail() {
            continue;
        }
        let consume = on_sucess_query.contains(stack.id)
            || (result.is_hit() && on_hit_query.contains(stack.id));
        if consume {
            if let Some(slot_num) = inventory_slot {
                if let Ok(mut inv) = inventory_query.get_mut(*user) {
                    inv.drop_items(*slot_num, 1);
                }
            }
        }
    }
}
