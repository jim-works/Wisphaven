use bevy::prelude::*;
use engine::items::{
    block_item::BlockItem, item_attributes::ConsumeItemOnHit, loot::CachedLootTable, CreatorItem,
    ItemBundle, ItemName, ItemResources, ItemStack, MaxStackSize, SpawnDroppedItemEvent,
};

use interfaces::scheduling::LevelSystemSet;
use rand::thread_rng;
use world::{
    block::{BlockId, BlockName, SingleBlockMesh},
    events::BlockBrokenEvent,
};

use crate::item_mesher::{ItemMesh, ItemMeshMaterial};
pub struct BlockItemsPlugin;

impl Plugin for BlockItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            create_block_item.run_if(resource_exists::<ItemResources>),
        )
        .add_systems(FixedUpdate, do_block_drops.in_set(LevelSystemSet::PostTick));
    }
}

fn create_block_item(
    mut items: ResMut<ItemResources>,
    block_query: Query<
        (Entity, &BlockName, Option<&SingleBlockMesh>),
        (Added<BlockId>, Without<CreatorItem>),
    >,
    mut commands: Commands,
) {
    for (entity, name, item_mesh) in block_query.iter() {
        let item_name = ItemName::core(name.name.clone());
        #[allow(state_scoped_entities)]
        let item = commands
            .spawn((
                ItemBundle {
                    name: item_name.clone(),
                    max_stack_size: MaxStackSize(999),
                },
                BlockItem(entity),
                ConsumeItemOnHit,
            ))
            .id();
        items.registry.add_basic(item_name, item, &mut commands);
        commands.entity(entity).insert(CreatorItem(item));
        if let Some(mesh) = item_mesh {
            info!("added item mesh for {:?}", name);
            commands.entity(item).insert(ItemMesh {
                mesh: mesh.0.clone(),
                material: ItemMeshMaterial::TextureArray,
            });
        }
    }
}

fn do_block_drops(
    block_query: Query<(&CreatorItem, Option<&CachedLootTable<Entity>>)>,
    mut reader: EventReader<BlockBrokenEvent>,
    mut drop_writer: EventWriter<SpawnDroppedItemEvent>,
) {
    let mut rng = thread_rng();
    for BlockBrokenEvent {
        block,
        coord,
        broken_by: _,
    } in reader.read()
    {
        match block_query.get(*block) {
            Ok((_, Some(loot_table))) => {
                info!("dropping from loot table");
                loot_table.drop_items(coord.center(), &mut drop_writer, &mut rng);
            }
            Ok((CreatorItem(creator_item), None)) => {
                info!("dropping from creator item");
                let random_v = util::sample_sphere_surface(&mut rng) * 0.05;
                let random_strength = util::random_proportion(&mut rng) + 0.5;
                drop_writer.send(SpawnDroppedItemEvent {
                    postion: coord.center(),
                    velocity: random_strength * (random_v + Vec3::Y * 0.1),
                    stack: ItemStack::new(*creator_item, 1),
                });
            }
            Err(_) => warn!("no block drop for {:?}", block),
        }
    }
}
