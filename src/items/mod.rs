use bevy::prelude::*;

use crate::world::LevelSystemSet;

pub mod inventory;
pub mod block_item;
pub mod weapons;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<UseItemEvent>()
            .add_event::<EquipItemEvent>()
            .add_event::<UnequipItemEvent>()
            .add_event::<PickupItemEvent>()
            .add_event::<DropItemEvent>()
            .add_event::<AttackItemEvent>()
            .add_system(block_item::use_block_item.in_set(LevelSystemSet::Main))
            .add_system(block_item::use_mega_block_item.in_set(LevelSystemSet::Main))
            .add_system(weapons::equip_unequip_weapon.in_set(LevelSystemSet::Main))
            .add_system(weapons::attack_melee.in_set(LevelSystemSet::Main))
        ;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub id: Entity,
    pub size: u32,
}
impl ItemStack {
    pub(crate) fn new(id: Entity, size: u32) -> ItemStack {
        Self {id, size}
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Component)]
pub struct Item {
    pub name: &'static str,
    pub max_stack_size: u32
}
impl Item {
    pub fn new(name: &'static str, max_stack_size: u32) -> Self {
        Self {
            name,
            max_stack_size
        }
    }
}

pub fn create_item<T: Bundle>(info: Item, bundle: T, commands: &mut Commands) -> Entity {
    commands.spawn(
        (info,
        bundle)
    ).id()
}

pub struct UseItemEvent(pub Entity, pub ItemStack, pub GlobalTransform);
pub struct AttackItemEvent(pub Entity, pub ItemStack, pub GlobalTransform);
pub struct EquipItemEvent(pub Entity, pub ItemStack);
pub struct UnequipItemEvent(pub Entity, pub ItemStack);
pub struct PickupItemEvent(pub Entity, pub ItemStack);
pub struct DropItemEvent(pub Entity, pub ItemStack);