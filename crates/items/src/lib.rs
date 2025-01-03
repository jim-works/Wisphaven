use bevy::prelude::*;

mod actor_items;
mod debug;
mod grapple_item;
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
        ));
    }
}
