use bevy::{prelude::*, utils::HashMap};

use crate::world::BlockType;

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
            .insert_resource(ItemRegistry::default())
            .add_system(block_item::use_block_item)
            .add_system(block_item::use_mega_block_item)
            .add_system(weapons::equip_unequip_weapon.in_base_set(CoreSet::PostUpdate))
            .add_system(weapons::attack_dagger)
        ;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub id: ItemType,
    pub size: u32,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum ItemType {
    Pickaxe,
    Dagger,
    Block(BlockType),
    MegaBlock(BlockType, i32),
}

pub struct UseItemEvent(pub Entity, pub ItemType, pub GlobalTransform);
pub struct AttackItemEvent(pub Entity, pub ItemStack, pub GlobalTransform);
pub struct EquipItemEvent(pub Entity, pub ItemStack);
pub struct UnequipItemEvent(pub Entity, pub ItemStack);
pub struct PickupItemEvent(pub Entity, pub ItemStack);
pub struct DropItemEvent(pub Entity, pub ItemStack);

#[derive(Resource)]
pub struct ItemRegistry {
    data: HashMap<ItemType, ItemData>,
    default_data: ItemData //temporary
}

pub struct ItemData {
    pub name: String,
    pub max_stack_size: u32
}

impl Default for ItemRegistry {
    fn default() -> Self {
        Self { 
            data: HashMap::new(),
            default_data: ItemData { name: "Default".to_string(), max_stack_size: 100 }
         }
    }
}

impl ItemRegistry {
    pub fn get_data(&self, item: &ItemType) -> Option<&ItemData> {
        Some(&self.default_data)
        //self.data.get(item)
    }
    pub fn set_data(&mut self, item: ItemType, data: ItemData) {
        self.data.insert(item, data);
    }
}