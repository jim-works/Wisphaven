use bevy::prelude::*;

use crate::{world::{Level, BlockCoord, BlockResources, BlockName, BlockId, events::ChunkUpdatedEvent, BlockType, BlockPhysics}, physics::{query::{RaycastHit, Ray, self}, collision::Aabb}};

use super::{UseHitEvent, UseItemEvent};

#[derive(Component)]
pub struct BlockItem(pub Entity);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct MegaBlockItem(pub BlockName, pub i32);

pub fn use_block_entity_item(
    mut reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseHitEvent>,
    block_query: Query<&BlockItem>,
    level: Res<Level>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    id_query: Query<&BlockId>,
    mut commands: Commands,
) {
    for UseItemEvent { user, inventory_slot, stack, tf } in reader.read() {
        if let Ok(block_item) = block_query.get(stack.id) {
            if let Some(RaycastHit::Block(coord, hit)) = query::raycast(
                Ray::new(tf.translation, tf.forward(), 10.0),
                &level,
                &block_physics_query,
                &object_query,
                vec![*user]
            ) {
                let normal = crate::util::max_component_norm(hit.hit_pos - coord.center()).into();
                level.set_block_entity(coord+normal, BlockType::Filled(block_item.0), &id_query, &mut update_writer, &mut commands);
                hit_writer.send(UseHitEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    pos: Some(hit.hit_pos),
                    success: true
                })
            } else {
                hit_writer.send(UseHitEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    pos: None,
                    success: false
                })
            }
        }
    }
}

pub fn use_mega_block_item(
    mut reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseHitEvent>,
    megablock_query: Query<&MegaBlockItem>,
    level: Res<Level>,
    resources: Res<BlockResources>,
    id_query: Query<&BlockId>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    mut commands: Commands,
) {
    for UseItemEvent { user, inventory_slot, stack, tf } in reader.read() {
        if let Ok(block_item) = megablock_query.get(stack.id) {
            let id = resources.registry.get_id(&block_item.0);
            let size = block_item.1;
            if let Some(RaycastHit::Block(coord, hit)) = query::raycast(
                Ray::new(tf.translation, tf.forward(), 10.0),
                &level,
                &block_physics_query,
                &object_query,
                vec![*user]
            ) {
                let mut changes = Vec::with_capacity((size*size*size) as usize);
                for x in -size..size+1 {
                    for y in -size..size+1 {
                        for z in -size..size+1 {
                            changes.push((
                                coord + BlockCoord::new(x, y, z),
                                id,
                            ));
                        }
                    }
                }
                level.batch_set_block(changes.into_iter(), &resources.registry, &id_query, &mut update_writer, &mut commands);
                hit_writer.send(UseHitEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    pos: Some(hit.hit_pos),
                    success: true
                })
            } else {
                hit_writer.send(UseHitEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    pos: None,
                    success: false
                })
            }
        }
    }
}