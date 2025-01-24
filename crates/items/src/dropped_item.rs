use std::time::Duration;

use bevy::prelude::*;
use engine::{
    actors::LocalPlayer,
    items::{
        inventory::Inventory, DroppedItem, DroppedItemPickerUpper, ItemName, ItemResources,
        ItemStack, MaxStackSize, SpawnDroppedItemEvent,
    },
    mesher::item_mesher::{HeldItemResources, ItemMesh},
    physics::{collision::Aabb, movement::Velocity, PhysicsBundle},
    world::LevelSystemSet,
};

pub(crate) struct DroppedItemPlugin;

impl Plugin for DroppedItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                spawn_dropped_item,
                activate_dropped_item,
                pickup_dropped_item,
            )
                .chain()
                .in_set(LevelSystemSet::PostTick),
        );
        app.add_systems(Update, test_spawning.in_set(LevelSystemSet::Main));
    }
}

#[derive(Component)]
struct InactiveDroppedItem {
    becomes: DroppedItem,
    at: Duration,
}

fn spawn_dropped_item(
    mut reader: EventReader<SpawnDroppedItemEvent>,
    mut commands: Commands,
    held_item_resources: Res<HeldItemResources>,
    item_query: Query<&ItemMesh>,
    time: Res<Time>,
) {
    //note: this code is kinda duplicated for the held item visual `visualize_held_item` but I think it's fine
    const SCALE: f32 = 0.5;
    let inactive_duration: Duration = Duration::from_secs_f32(0.5);
    for spawn in reader.read() {
        let mut ec = commands.spawn((
            Transform::from_translation(spawn.postion).with_scale(Vec3::splat(SCALE)),
            PhysicsBundle {
                velocity: Velocity(spawn.velocity),
                collider: Aabb::new(Vec3::splat(SCALE), Vec3::ZERO),
                ..default()
            },
            InactiveDroppedItem {
                becomes: DroppedItem { stack: spawn.stack },
                at: time.elapsed() + inactive_duration,
            },
        ));
        let mesh = if let Ok(item_mesh) = item_query.get(spawn.stack.id) {
            match item_mesh.material {
                engine::mesher::item_mesher::ItemMeshMaterial::ColorArray => {
                    ec.insert(MeshMaterial3d(held_item_resources.color_material.clone()))
                }
                engine::mesher::item_mesher::ItemMeshMaterial::TextureArray => {
                    ec.insert(MeshMaterial3d(held_item_resources.texture_material.clone()))
                }
            };
            item_mesh.mesh.clone()
        } else {
            Default::default()
        };
        ec.insert(Mesh3d(mesh));
    }
}

fn activate_dropped_item(
    query: Query<(Entity, &InactiveDroppedItem)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let current = time.elapsed();
    for (entity, dropped) in query.iter() {
        if current >= dropped.at {
            commands
                .entity(entity)
                .remove::<InactiveDroppedItem>()
                .insert(dropped.becomes);
        }
    }
}

fn pickup_dropped_item(
    mut picker_upper_query: Query<
        (&mut Inventory, &DroppedItemPickerUpper, &Transform),
        Without<DroppedItem>,
    >,
    mut dropped_item_query: Query<
        (Entity, &mut DroppedItem, &Transform),
        Without<DroppedItemPickerUpper>,
    >,
    stack_query: Query<&MaxStackSize>,
    mut commands: Commands,
) {
    // todo - optimize spatial query
    for (mut inv, picker_upper, picker_upper_tf) in picker_upper_query.iter_mut() {
        for (dropped_entity, mut dropped_item, dropped_tf) in dropped_item_query.iter_mut() {
            if picker_upper_tf
                .translation
                .distance_squared(dropped_tf.translation)
                <= picker_upper.radius * picker_upper.radius
            {
                if let Some(remaining_items) = inv.pickup_item(dropped_item.stack, &stack_query) {
                    dropped_item.stack = remaining_items;
                } else {
                    //picked up all items, despawn stack
                    commands.entity(dropped_entity).despawn();
                }
            }
        }
    }
}

fn test_spawning(
    button: Res<ButtonInput<KeyCode>>,
    mut writer: EventWriter<SpawnDroppedItemEvent>,
    player_query: Query<&Transform, With<LocalPlayer>>,
    items: Res<ItemResources>,
) {
    let Ok(player) = player_query.get_single() else {
        return;
    };
    let pick = items
        .registry
        .get_basic(&ItemName::core("ruby_pickaxe"))
        .unwrap();
    let log = items.registry.get_basic(&ItemName::core("log")).unwrap();
    if button.just_pressed(KeyCode::KeyT) {
        writer.send(SpawnDroppedItemEvent {
            postion: player.translation,
            velocity: 0.2 * (player.forward().as_vec3() + Vec3::Y),
            stack: ItemStack::new(pick, 5),
        });
    }
    if button.just_pressed(KeyCode::KeyY) {
        writer.send(SpawnDroppedItemEvent {
            postion: player.translation,
            velocity: 0.2 * (player.forward().as_vec3() + Vec3::Y),
            stack: ItemStack::new(log, 5),
        });
    }
}
