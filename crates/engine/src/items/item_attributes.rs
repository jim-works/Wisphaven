use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{UseEndEvent, inventory::Inventory};
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

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Component,
    Reflect,
    Default,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
)]
#[reflect(Component, FromWorld)]
pub struct ItemSwingSpeed(pub Duration);

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Component,
    Reflect,
    Default,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
)]
#[reflect(Component, FromWorld)]
pub struct ItemUseSpeed(pub Duration);

fn consume_items(
    mut events: EventReader<UseEndEvent>,
    on_hit_query: Query<&ConsumeItemOnHit>,
    on_sucess_query: Query<&ConsumeItemOnSucess>,
    mut inventory_query: Query<&mut Inventory>,
) {
    for UseEndEvent {
        user, slot, result, ..
    } in events.read()
    {
        if result.is_fail() {
            continue;
        }
        if let Some((slot, stack)) = slot {
            let consume = on_sucess_query.contains(stack.id)
                || (result.is_hit() && on_hit_query.contains(stack.id));
            if consume {
                if let Ok(mut inv) = inventory_query.get_mut(*user) {
                    inv.drop_items(*slot, 1);
                }
            }
        }
    }
}
