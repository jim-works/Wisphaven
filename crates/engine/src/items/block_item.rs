use bevy::prelude::*;

use crate::{
    physics::{
        collision::Aabb,
        query::{self, Raycast, RaycastHit},
    },
    world::{events::ChunkUpdatedEvent, BlockId, BlockPhysics, BlockType, Level},
};

use super::{HitResult, UseEndEvent, UseItemEvent};

#[derive(Component)]
pub struct BlockItem(pub Entity);

pub fn use_block_entity_item(
    mut reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseEndEvent>,
    block_query: Query<&BlockItem>,
    level: Res<Level>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    id_query: Query<&BlockId>,
    mut commands: Commands,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf,
    } in reader.read()
    {
        if let Ok(block_item) = block_query.get(stack.id) {
            if let Some(RaycastHit::Block(coord, hit)) = query::raycast(
                Raycast::new(tf.translation, tf.forward(), 10.0),
                &level,
                &block_physics_query,
                &object_query,
                &[*user],
            ) {
                let normal = crate::util::max_component_norm(hit.hit_pos - coord.center()).into();
                level.set_block_entity(
                    coord + normal,
                    BlockType::Filled(block_item.0),
                    &id_query,
                    &mut update_writer,
                    &mut commands,
                );
                hit_writer.send(UseEndEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    result: HitResult::Hit(hit.hit_pos),
                });
            } else {
                hit_writer.send(UseEndEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    result: HitResult::Miss,
                });
            }
        }
    }
}
