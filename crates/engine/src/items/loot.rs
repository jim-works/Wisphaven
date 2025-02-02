use bevy::prelude::*;

use rand::prelude::*;
use rand_distr::Uniform;
use serde::{Deserialize, Serialize};
use util::random_proportion;

use interfaces::scheduling::LevelSystemSet;

use super::{ItemName, ItemRegistry, ItemResources, ItemStack, SpawnDroppedItemEvent};

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cache_item_loot_table.in_set(LevelSystemSet::Main));
        app.register_type::<ItemLootTable>();
    }
}

#[derive(Default, Clone, Debug, PartialEq, Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, FromWorld)]
pub struct ItemLootTable {
    //everything is dropped if a drop occurs
    pub drops: Vec<ItemLootTableDrop>,
}

impl ItemLootTable {
    pub fn cache(&self, items: &ItemRegistry) -> CachedLootTable<Entity> {
        CachedLootTable {
            drops: self
                .drops
                .iter()
                .flat_map(|drop| {
                    items.get_basic(&drop.item).map(|item| CachedLootTableDrop {
                        item,
                        drop_chance: drop.drop_chance,
                        drop_count_range: drop.drop_count_range,
                    })
                })
                .collect(),
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Component, Reflect, Serialize, Deserialize)]
pub struct ItemLootTableDrop {
    pub item: ItemName,
    pub drop_chance: f32,
    //inclusive
    pub drop_count_range: (u32, u32),
}

pub trait LootTableDroppable: Send + Sync + Clone {}
impl<T: Send + Sync + Clone> LootTableDroppable for T {}

#[derive(Component)]
pub struct CachedLootTable<T: LootTableDroppable> {
    pub drops: Vec<CachedLootTableDrop<T>>,
}

pub struct CachedLootTableDrop<T> {
    pub item: T,
    pub drop_chance: f32,
    //inclusive
    pub drop_count_range: (u32, u32),
}

impl<T: LootTableDroppable> CachedLootTable<T> {
    pub fn get_loot(&self) -> impl Iterator<Item = (T, u32)> + use<'_, T> {
        let mut rng = thread_rng();
        self.drops.iter().flat_map(move |drop| {
            let prop = random_proportion(&mut rng);
            info!(
                "this is the drop chance {:?} this is the roll {:?}",
                drop.drop_chance, prop
            );
            if prop > drop.drop_chance {
                None
            } else {
                Some((
                    drop.item.clone(),
                    Uniform::new_inclusive(drop.drop_count_range.0, drop.drop_count_range.1)
                        .sample(&mut rng),
                ))
            }
        })
    }
}

impl CachedLootTable<Entity> {
    pub fn drop_items(
        &self,
        position: Vec3,
        drop_writer: &mut EventWriter<SpawnDroppedItemEvent>,
        rng: &mut impl Rng,
    ) {
        for (item, size) in self.get_loot() {
            let random_v = util::sample_sphere_surface(rng) * 0.05;
            let random_strength = util::random_proportion(rng) + 0.5;
            drop_writer.send(SpawnDroppedItemEvent {
                postion: position,
                velocity: random_strength * (random_v + Vec3::Y * 0.1),
                stack: ItemStack::new(item, size),
            });
        }
    }
}

fn cache_item_loot_table(
    mut commands: Commands,
    query: Query<(Entity, &ItemLootTable), Without<CachedLootTable<Entity>>>,
    items: Res<ItemResources>,
) {
    for (entity, table) in query.iter() {
        let Some(mut ec) = commands.get_entity(entity) else {
            continue;
        };
        ec.insert(table.cache(&items.registry));
    }
}
