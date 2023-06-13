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
            .add_system(block_item::use_mega_block_item)
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
    Block(BlockType),
    MegaBlock(BlockType, i32),
}

pub struct UseItemEvent(Entity, ItemType, GlobalTransform);
pub struct EquipItemEvent(Entity, ItemStack);
pub struct UnequipItemEvent(Entity, ItemStack);
pub struct PickupItemEvent(Entity, ItemStack);
pub struct DropItemEvent(Entity, ItemStack);

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