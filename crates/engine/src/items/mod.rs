use std::{path::PathBuf, sync::Arc};

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

use interfaces::components::Id;
use interfaces::scheduling::ItemSystemSet;

use self::item_attributes::ItemAttributesPlugin;

pub mod block_item;
pub mod inventory;
pub mod item_attributes;
pub mod loot;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StartUsingItemEvent>()
            .add_event::<UseItemEvent>()
            .add_event::<UseEndEvent>()
            .add_event::<StartSwingingItemEvent>()
            .add_event::<SwingItemEvent>()
            .add_event::<SwingEndEvent>()
            .add_event::<SpawnDroppedItemEvent>()
            .add_plugins((ItemAttributesPlugin, loot::LootPlugin))
            .add_systems(
                Update,
                (block_item::use_block_entity_item,).in_set(ItemSystemSet::UsageProcessing),
            )
            .add_systems(
                Update,
                inventory::tick_item_timers.in_set(ItemSystemSet::Usage),
            )
            .register_type::<NamedItemIcon>()
            .register_type::<MaxStackSize>()
            .register_type::<ItemName>();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub id: Entity,
    pub size: u32,
}
impl ItemStack {
    pub fn new(id: Entity, size: u32) -> ItemStack {
        Self { id, size }
    }
}

#[derive(
    Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component, FromWorld)]
pub struct ItemName {
    pub namespace: String,
    pub name: String,
}

#[derive(
    Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component, FromWorld)]
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
#[reflect(Component, FromWorld)]
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
    #[allow(state_scoped_entities)]
    commands.spawn((info, bundle)).id()
}

#[derive(Component)]
pub struct ItemIcon(pub Handle<Image>);

#[derive(Component, Clone, Copy)]
pub struct DroppedItem {
    pub stack: ItemStack,
}

#[derive(Component, Clone, Copy)]
pub struct DroppedItemPickerUpper {
    pub radius: f32,
}

#[derive(Event)]
pub struct SpawnDroppedItemEvent {
    pub postion: Vec3,
    pub velocity: Vec3,
    pub stack: ItemStack,
}

#[derive(Event)]
pub struct StartUsingItemEvent {
    pub user: Entity,
    pub inventory_slot: Option<usize>,
    pub stack: ItemStack,
    pub tf: Transform,
}

#[derive(Event)]
pub struct UseItemEvent {
    pub user: Entity,
    pub inventory_slot: Option<usize>,
    pub stack: ItemStack,
    pub tf: Transform,
}

#[derive(Event)]
pub struct UseEndEvent {
    pub user: Entity,
    pub inventory_slot: Option<usize>,
    pub stack: ItemStack,
    pub result: HitResult,
}

#[derive(Event)]
pub struct StartSwingingItemEvent {
    pub user: Entity,
    pub inventory_slot: Option<usize>,
    pub stack: ItemStack,
    pub tf: Transform,
}

#[derive(Event)]
pub struct SwingItemEvent {
    pub user: Entity,
    pub inventory_slot: Option<usize>,
    pub stack: ItemStack,
    pub tf: Transform,
}

#[derive(Event)]
pub struct SwingEndEvent {
    pub user: Entity,
    pub inventory_slot: Option<usize>,
    pub stack: ItemStack,
    pub result: HitResult,
}

#[derive(Clone, Copy)]
pub enum HitResult {
    Hit(Vec3),
    Miss,
    Fail,
}

impl HitResult {
    pub fn is_hit(self) -> bool {
        matches!(self, HitResult::Hit(_))
    }
    pub fn is_miss(self) -> bool {
        matches!(self, HitResult::Miss)
    }
    pub fn is_fail(self) -> bool {
        matches!(self, HitResult::Fail)
    }
}

#[derive(Component)]
// place on blocks, actors, etc to denote that they are placed/spawned by the referenced item entity
pub struct CreatorItem(pub Entity);

#[derive(Resource, Default)]
pub struct ItemResources {
    pub registry: ItemRegistry,
    pub loaded: bool,
}

pub type ItemNameIdMap = HashMap<ItemName, ItemId>;

//similar to BlockGenerator
pub trait ItemGenerator: Send + Sync {
    fn generate(&self, item: Entity, commands: &mut Commands);
}

#[derive(Default)]
pub struct ItemRegistry {
    pub basic_entities: Vec<Entity>,
    pub dynamic_generators: Vec<Arc<dyn ItemGenerator>>,
    //item ids may not be stable across program runs
    pub id_map: ItemNameIdMap,
}

impl ItemRegistry {
    //inserts the corresponding id component on the item
    pub fn add_basic(&mut self, name: ItemName, entity: Entity, commands: &mut Commands) {
        info!("added id {:?}", name);
        let id = ItemId(Id::Basic(self.basic_entities.len() as u32));
        commands.entity(entity).insert(id);
        self.basic_entities.push(entity);
        if self.id_map.contains_key(&name) {
            error!("duplicate item: {:?}", name);
            #[cfg(debug_assertions)]
            panic!("duplicate item: {:?}", name);
        }
        self.id_map.insert(name, id);
    }
    pub fn add_dynamic(&mut self, name: ItemName, generator: Arc<dyn ItemGenerator>) {
        let id = ItemId(Id::Dynamic(self.dynamic_generators.len() as u32));
        self.dynamic_generators.push(generator);
        self.id_map.insert(name, id);
    }
    pub fn create_basic(&mut self, bundle: ItemBundle, commands: &mut Commands) -> Entity {
        let name = bundle.name.clone();
        #[allow(state_scoped_entities)]
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
                error!("Couldn't find item id for name {:?}", name);
                ItemId(Id::Empty)
            }
        }
    }
    pub fn get_entity(&self, item_id: ItemId, commands: &mut Commands) -> Option<Entity> {
        match item_id {
            ItemId(Id::Empty) => None,
            ItemId(Id::Basic(id)) => self.basic_entities.get(id as usize).copied(),
            ItemId(Id::Dynamic(id)) => self.dynamic_generators.get(id as usize).map(|generator| {
                let id = Self::setup_item(item_id, commands);
                generator.generate(id, commands);
                id
            }),
        }
    }
    fn setup_item(id: ItemId, commands: &mut Commands) -> Entity {
        #[allow(state_scoped_entities)]
        commands.spawn(id).id()
    }
}
