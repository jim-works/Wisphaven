use bevy::prelude::*;

mod actor_items;
pub mod block_items;
mod debug;
mod dropped_item;
mod grapple_item;
pub mod item_mesher;
mod time_items;
mod tools;
mod weapons;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            weapons::WeaponItemPlugin,
            time_items::TimeItemsPlugin,
            grapple_item::GrappleItemPlugin,
            actor_items::ActorItemsPlugin,
            tools::ToolsPlugin,
            debug::DebugItems,
            dropped_item::DroppedItemPlugin,
            item_mesher::ItemMesherPlugin,
            block_items::BlockItemsPlugin,
        ));
    }
}
