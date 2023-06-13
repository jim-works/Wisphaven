use bevy::{prelude::*, utils::HashMap};

use crate::world::BlockType;

pub mod inventory;
pub mod block_item;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<UseItemEvent>()
            .add_event::<EquipItemEvent>()
            .add_event::<UnequipItemEvent>()
            .add_event::<PickupItemEvent>()
            .add_event::<DropItemEvent>()
            .insert_resource(ItemRegistry::default())
            .add_system(block_item::use_block_item)
        ;
    }
}

#[derive(Clone)]
pub struct ItemStack {
    pub id: ItemType,
    pub size: u32,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum ItemType {
    Pickaxe,
    Gun,
    Block(BlockType)
}

pub struct UseItemEvent(Entity, ItemType, GlobalTransform);
pub struct EquipItemEvent(Entity, ItemStack);
pub struct UnequipItemEvent(Entity, ItemStack);
pub struct PickupItemEvent(Entity, ItemStack);
pub struct DropItemEvent(Entity, ItemStack);

#[derive(Resource)]
pub struct ItemRegistry {
    data: HashMap<ItemType, ItemData>
}

pub struct ItemData {
    pub name: String,
    pub max_stack_size: u32
}

impl Default for ItemRegistry {
    fn default() -> Self {
        Self { 
            data: HashMap::new()
         }
    }
}

impl ItemRegistry {
    pub fn get_data(&self, item: &ItemType) -> Option<&ItemData> {
        self.data.get(item)
    }
    pub fn set_data(&mut self, item: ItemType, data: ItemData) {
        self.data.insert(item, data);
    }
}