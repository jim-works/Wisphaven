use std::{path::PathBuf, sync::Arc};

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

use crate::world::{Id, LevelSystemSet};

use self::item_attributes::ItemAttributesPlugin;

pub mod actor_items;
pub mod block_item;
pub mod crafting;
pub mod debug_items;
pub mod inventory;
pub mod item_attributes;
pub mod loot;
pub mod time_items;
pub mod tools;
pub mod weapons;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<UseItemEvent>()
            .add_event::<EquipItemEvent>()
            .add_event::<UnequipItemEvent>()
            .add_event::<PickupItemEvent>()
            .add_event::<DropItemEvent>()
            .add_event::<SwingItemEvent>()
            .configure_sets(
                Update,
                (
                    ItemSystemSet::Usage.in_set(LevelSystemSet::Main),
                    ItemSystemSet::UsageProcessing.in_set(LevelSystemSet::Main),
                    ItemSystemSet::DropPickup.in_set(LevelSystemSet::Main),
                    ItemSystemSet::DropPickupProcessing.in_set(LevelSystemSet::Main),
                ).chain(),
            )
            .add_plugins((
                debug_items::DebugItems,
                tools::ToolsPlugin,
                actor_items::ActorItems,
                weapons::WeaponItemPlugin,
                ItemAttributesPlugin,
                time_items::TimeItemsPlugin,
                loot::LootPlugin,
                crafting::CraftingPlugin,
            ))
            .add_systems(
                Update,
                (
                    block_item::use_block_entity_item,
                    block_item::use_mega_block_item,
                )
                    .in_set(ItemSystemSet::UsageProcessing),
            )
            .add_systems(
                Update,
                inventory::tick_item_timers.in_set(ItemSystemSet::Usage),
            )
            .register_type::<NamedItemIcon>()
            .register_type::<ItemName>()
            .register_type::<weapons::MeleeWeaponItem>()
            .register_type::<block_item::MegaBlockItem>();
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum ItemSystemSet {
    Usage,
    UsageProcessing,
    DropPickup,
    DropPickupProcessing,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub id: Entity,
    pub size: u32,
}
impl ItemStack {
    pub(crate) fn new(id: Entity, size: u32) -> ItemStack {
        Self { id, size }
    }
}

#[derive(
    Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component)]
pub struct ItemName {
    pub namespace: String,
    pub name: String,
}

#[derive(
    Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component)]
pub struct NamedItemIcon {
    pub path: PathBuf,
}

impl ItemName {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }
    pub fn core(name: impl Into<String>) -> Self {
        Self::new("core", name)
    }
}

#[derive(Bundle)]
pub struct ItemBundle {
    pub name: ItemName,
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

pub fn create_item<T: Bundle>(
    info: ItemBundle,
    icon: ItemIcon,
    bundle: T,
    commands: &mut Commands,
) -> Entity {
    create_raw_item(info, (icon, bundle), commands)
}

//lessens the requirements for an item (for example without an icon)
pub fn create_raw_item<T: Bundle>(info: ItemBundle, bundle: T, commands: &mut Commands) -> Entity {
    commands.spawn((info, bundle)).id()
}

#[derive(Component)]
pub struct ItemIcon(pub Handle<Image>);

#[derive(Event)]
pub struct UseItemEvent {
    pub user: Entity,
    pub inventory_slot: usize,
    pub stack: ItemStack,
    pub tf: GlobalTransform,
}
#[derive(Event)]
pub struct SwingItemEvent {
    pub user: Entity,
    pub inventory_slot: usize,
    pub stack: ItemStack,
    pub tf: GlobalTransform,
}
#[derive(Event)]
pub struct EquipItemEvent {
    pub user: Entity,
    pub inventory_slot: usize,
    pub stack: ItemStack,
}
#[derive(Event)]
pub struct UnequipItemEvent {
    pub user: Entity,
    pub inventory_slot: usize,
    pub stack: ItemStack,
}
#[derive(Event)]
pub struct PickupItemEvent {
    pub user: Entity,
    //no slot because we can pick up items into multiple slots at once
    pub stack: ItemStack,
}
#[derive(Event)]
pub struct DropItemEvent {
    pub user: Entity,
    pub inventory_slot: usize,
    pub stack: ItemStack,
}

#[derive(Component)]
// place on blocks, actors, etc to denote that they are placed/spawned by the referenced item entity
pub struct CreatorItem(pub Entity);

#[derive(Resource)]
pub struct ItemResources {
    pub registry: Arc<ItemRegistry>,
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
    pub id_map: ItemNameIdMap,
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
    pub fn create_basic(&mut self, bundle: ItemBundle, commands: &mut Commands) -> Entity {
        let name = bundle.name.clone();
        let entity = commands.spawn(bundle).id();
        self.add_basic(name, entity, commands);
        entity
    }
    pub fn get_basic(&self, name: &ItemName) -> Option<Entity> {
        let id = self.id_map.get(name)?;
        match id {
            ItemId(Id::Basic(id)) => self.basic_entities.get(*id as usize).copied(),
            _ => None,
        }
    }
    pub fn get_id(&self, name: &ItemName) -> ItemId {
        match self.id_map.get(name) {
            Some(id) => *id,
            None => {
                error!("Couldn't find block id for name {:?}", name);
                ItemId(Id::Empty)
            }
        }
    }
    pub fn get_entity(&self, item_id: ItemId, commands: &mut Commands) -> Option<Entity> {
        match item_id {
            ItemId(Id::Empty) => None,
            ItemId(Id::Basic(id)) => self.basic_entities.get(id as usize).copied(),
            ItemId(Id::Dynamic(id)) => self.dynamic_generators.get(id as usize).map(|gen| {
                let id = Self::setup_item(item_id, commands);
                gen.generate(id, commands);
                id
            }),
        }
    }
    fn setup_item(id: ItemId, commands: &mut Commands) -> Entity {
        commands.spawn(id).id()
    }
}
