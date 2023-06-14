use bevy::{prelude::*, utils::HashMap};

use crate::world::BlockType;

use self::weapons::MeleeWeaponItem;

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
            .add_startup_system(init)
            .add_system(block_item::use_block_item)
            .add_system(block_item::use_mega_block_item)
            .add_system(weapons::equip_unequip_weapon.in_base_set(CoreSet::PostUpdate))
            .add_system(weapons::attack_melee)
        ;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub id: Entity,
    pub size: u32,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Component)]
pub enum ItemType {
    Pickaxe,
    Dagger,
    Block(BlockType),
    MegaBlock(BlockType, i32),
}

pub struct UseItemEvent(pub Entity, pub ItemStack, pub GlobalTransform);
pub struct AttackItemEvent(pub Entity, pub ItemStack, pub GlobalTransform);
pub struct EquipItemEvent(pub Entity, pub ItemStack);
pub struct UnequipItemEvent(pub Entity, pub ItemStack);
pub struct PickupItemEvent(pub Entity, pub ItemStack);
pub struct DropItemEvent(pub Entity, pub ItemStack);

#[derive(Resource)]
pub struct ItemRegistry {
    entities: HashMap<ItemType, Entity>,
    default_data: ItemData //temporary
}

#[derive(Component)]
pub struct ItemData {
    pub name: String,
    pub max_stack_size: u32
}
impl ItemData {
    pub fn new(name: String, max_stack_size: u32) -> Self {
        Self {
            name,
            max_stack_size
        }
    }
}

impl ItemRegistry {
    pub fn new(commands: &mut Commands) -> Self {
        let mut registry = Self {
            entities: HashMap::new(),
            default_data: ItemData { name: "Unknown Item".to_string(), max_stack_size: 100 }
        };
        registry.insert_item(ItemType::Dagger, ItemData::new("Dagger".to_string(), 1), MeleeWeaponItem {damage: 5.0, knockback: 0.5}, commands);
        registry
    }
    pub fn get_entity(&self, item: &ItemType) -> Option<Entity> {
        self.entities.get(item).copied()
    }
    pub fn get_stack(&self, item: &ItemType, size: u32) -> Option<ItemStack> {
        match self.get_entity(item) {
            Some(id) => Some(ItemStack { id, size }),
            None => None
        }
    }
    pub fn insert_item<T: Bundle>(&mut self, item_type: ItemType, item_data: ItemData, components: T, commands: &mut Commands) {
        let entity = commands.spawn((item_type, item_data, components));
        self.entities.insert(item_type, entity.id());
    }
}

fn init(mut commands: Commands) {
    commands.insert_resource(ItemRegistry::new(&mut commands));
}