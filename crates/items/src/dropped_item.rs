use std::time::Duration;

use crate::item_mesher::{ItemMesh, ItemMeshMaterial};
use bevy::prelude::*;
use dialogue::{DialogueEffect, DialogueEffectEvent};
use engine::items::{
    DroppedItem, DroppedItemPickerUpper, ItemName, ItemResources, ItemStack, MaxStackSize,
    SpawnDroppedItemEvent, inventory::Inventory,
};
use interfaces::{
    resources::HeldItemResources,
    scheduling::{GameState, LevelSystemSet},
};
use physics::{
    FrictionBundle, PhysicsBundle,
    collision::{Aabb, Friction},
    movement::Velocity,
};

pub(crate) struct DroppedItemPlugin;

impl Plugin for DroppedItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                handle_dialogue_effect,
                spawn_dropped_item,
                activate_dropped_item,
                pickup_dropped_item,
            )
                .chain()
                .in_set(LevelSystemSet::PostTick),
        );
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
            StateScoped(GameState::Game),
            Transform::from_translation(spawn.postion).with_scale(Vec3::splat(SCALE)),
            PhysicsBundle {
                velocity: Velocity(spawn.velocity),
                collider: Aabb::new(Vec3::splat(SCALE), Vec3::ZERO),
                friction: FrictionBundle {
                    // todo - @polish maybe this could be set according to the friction of the block? so ice slides far
                    friction: Friction(1.5),
                    ..default()
                },
                ..default()
            },
            InactiveDroppedItem {
                becomes: DroppedItem { stack: spawn.stack },
                at: time.elapsed() + inactive_duration,
            },
        ));
        let mesh = if let Ok(item_mesh) = item_query.get(spawn.stack.id) {
            match item_mesh.material {
                ItemMeshMaterial::ColorArray => {
                    ec.insert(MeshMaterial3d(held_item_resources.color_material.clone()))
                }
                ItemMeshMaterial::TextureArray => {
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

fn handle_dialogue_effect(
    position_query: Query<&GlobalTransform>,
    mut effect_reader: EventReader<DialogueEffectEvent>,
    mut drop_writer: EventWriter<SpawnDroppedItemEvent>,
    item_resources: Res<ItemResources>,
    mut commands: Commands,
) {
    for effect in effect_reader.read() {
        if let DialogueEffect::GiveItem {
            give_item: drop_item,
            quantity,
        } = &effect.effect
        {
            // try to give item to other person first, fallback to ourselves
            let gtf = match position_query.get(effect.other_entity) {
                Ok(gtf) => gtf,
                Err(_) => match position_query.get(effect.dialogue_entity) {
                    Ok(gtf) => gtf,
                    Err(_) => {
                        error!(
                            "cannot get position for entity to give item from dialgoue effect {:?}",
                            effect.dialogue_entity
                        );
                        continue;
                    }
                },
            };
            match ItemName::try_from(drop_item.as_ref()) {
                Ok(item_name) => {
                    match item_resources
                        .registry
                        .get_entity(item_resources.registry.get_id(&item_name), &mut commands)
                    {
                        Some(item) => {
                            info!("Dropping {} x{} for dialogue event", item_name, quantity);
                            drop_writer.send(SpawnDroppedItemEvent {
                                postion: gtf.translation(),
                                velocity: Vec3::ZERO,
                                stack: ItemStack::new(item, *quantity),
                            });
                        }
                        None => error!("invalid item name `{}`", item_name),
                    }
                }
                Err(e) => error!(
                    "Error parsing item name to drop from dialogue {}: {:?}",
                    drop_item, e
                ),
            }
        }
    }
}
