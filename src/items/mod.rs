use std::{sync::Arc, path::PathBuf};

use bevy::{prelude::*, utils::HashMap};
use serde::{Serialize, Deserialize};

use crate::world::{LevelSystemSet, Id};

pub mod inventory;
pub mod block_item;
pub mod weapons;
pub mod debug_items;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<UseItemEvent>()
            .add_event::<EquipItemEvent>()
            .add_event::<UnequipItemEvent>()
            .add_event::<PickupItemEvent>()
            .add_event::<DropItemEvent>()
            .add_event::<AttackItemEvent>()
            .add_plugin(debug_items::DebugItems)
            .add_system(block_item::use_block_item.in_set(LevelSystemSet::Main))
            .add_system(block_item::use_mega_block_item.in_set(LevelSystemSet::Main))
            .add_system(weapons::equip_unequip_weapon.in_set(LevelSystemSet::Main))
            .add_system(weapons::attack_melee.in_set(LevelSystemSet::Main))

            .register_type::<NamedItemIcon>()
            .register_type::<ItemName>()
            .register_type::<weapons::MeleeWeaponItem>()
            .register_type::<block_item::BlockItem>()
            .register_type::<block_item::MegaBlockItem>()
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

#[derive(Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ItemName {
    pub namespace: String,
    pub name: String,
}

#[derive(Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct NamedItemIcon {
    pub path: PathBuf
}

impl ItemName {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self { namespace: namespace.into(), name: name.into() }
    }
    pub fn core(name: impl Into<String>) -> Self {
        Self::new("core", name)
    }
}

#[derive(Bundle)]
pub struct ItemBundle {
    pub name: ItemName,
    pub icon: ItemIcon,
    pub max_stack_size: MaxStackSize,
}

//item ids may not be stable across program runs. to get a specific id for a item,
// use item registry
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub Id);

impl From<Id> for ItemId {
    fn from(value: Id) -> Self {
        Self(value)
    }
}

impl From<ItemId> for Id {
    fn from(value: ItemId) -> Self {
        value.0
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct MaxStackSize(pub u32);

pub fn create_item<T: Bundle>(info: ItemBundle, icon: ItemIcon, bundle: T, commands: &mut Commands) -> Entity {
    create_raw_item(info, (icon, bundle), commands)
}

//lessens the requirements for an item (for example without an icon)
pub fn create_raw_item<T: Bundle>(info: ItemBundle, bundle: T, commands: &mut Commands) -> Entity {
    commands.spawn(
        (info,
        bundle)
    ).id()
}

#[derive(Component)]
pub struct ItemIcon(pub Handle<Image>);

pub struct UseItemEvent(pub Entity, pub ItemStack, pub GlobalTransform);
pub struct AttackItemEvent(pub Entity, pub ItemStack, pub GlobalTransform);
pub struct EquipItemEvent(pub Entity, pub ItemStack);
pub struct UnequipItemEvent(pub Entity, pub ItemStack);
pub struct PickupItemEvent(pub Entity, pub ItemStack);
pub struct DropItemEvent(pub Entity, pub ItemStack);

#[derive(Resource)]
pub struct ItemResources {
    pub registry: Arc<ItemRegistry>
}

pub type ItemNameIdMap = HashMap<ItemName, ItemId>;

//similar to BlockGenerator
pub trait ItemGenerator: Send + Sync {
    fn generate(&self, item: Entity, commands: &mut Commands);
}

#[derive(Default)]
pub struct ItemRegistry {
    pub basic_entities: Vec<Entity>,
    pub dynamic_generators: Vec<Box<dyn ItemGenerator>>,
    //block ids may not be stable across program runs
    pub id_map: ItemNameIdMap
}

impl ItemRegistry {
    //inserts the corresponding BlockId component on the block
    pub fn add_basic(&mut self, name: ItemName, entity: Entity, commands: &mut Commands) {
        info!("added id {:?}", name);
        let id = ItemId(Id::Basic(self.basic_entities.len() as u32));
        commands.entity(entity).insert(id);
        self.basic_entities.push(entity);
        self.id_map.insert(name, id);
    }
    pub fn add_dynamic(&mut self, name: ItemName, generator: Box<dyn ItemGenerator>) {
        let id = ItemId(Id::Dynamic(self.dynamic_generators.len() as u32));
        self.dynamic_generators.push(generator);
        self.id_map.insert(name, id);
    }
    pub fn create_basic(&mut self, bundle: ItemBundle, commands: &mut Commands) -> Entity{
        let name = bundle.name.clone();
        let entity = commands.spawn(bundle).id();
        self.add_basic(name, entity, commands);
        entity
    }
    pub fn get_basic(&self, name: &ItemName) -> Option<Entity> {
        let id = self.id_map.get(&name)?;
        match id {
            ItemId(Id::Basic(id)) => self.basic_entities.get(*id as usize).copied(),
            _ => None
        }
    }
    pub fn get_id(&self, name: &ItemName) -> ItemId {
        match self.id_map.get(name) {
            Some(id) => *id,
            None => {
                error!("Couldn't find block id for name {:?}", name);
                ItemId(Id::Empty)
            },
        }
    }
    pub fn get_entity(&self, item_id: ItemId, commands: &mut Commands) -> Option<Entity> {
        match item_id {
            ItemId(Id::Empty) => None,
            ItemId(Id::Basic(id)) => self.basic_entities.get(id as usize).copied(),
            ItemId(Id::Dynamic(id)) => self.dynamic_generators.get(id as usize).and_then(|gen| {
                let id = Self::setup_item(item_id, commands);
                gen.generate(id, commands);
                Some(id)
            }),
        }
    }
    fn setup_item(id: ItemId, commands: &mut Commands) -> Entity {
        commands.spawn(id).id()
    }
}