use bevy::prelude::*;

use crate::{
    actors::personality::components::*,
    physics::{
        collision::Aabb,
        query::{raycast, Raycast, RaycastHit},
    },
    world::{BlockPhysics, Level},
};

use super::{HitResult, ItemSystemSet, UseEndEvent, UseItemEvent};

pub struct DebugItems;

impl Plugin for DebugItems {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            use_personality_item.in_set(ItemSystemSet::UsageProcessing),
        )
        .register_type::<PersonalityTester>();
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, FromWorld)]
pub struct PersonalityTester;

pub fn use_personality_item(
    mut reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseEndEvent>,
    physical_attributes: Query<&PhysicalAttributes>,
    mental_attributes: Query<&MentalAttributes>,
    values: Query<&PersonalityValues>,
    tasks: Query<&TaskSet>,
    personality_item: Query<&PersonalityTester>,
    level: Res<Level>,
    physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf,
    } in reader.read()
    {
        if personality_item.contains(stack.id) {
            if let Some(RaycastHit::Object(hit)) = raycast(
                Raycast::new(tf.translation, tf.forward(), 10.0),
                &level,
                &physics_query,
                &object_query,
                &[*user],
            ) {
                if let Ok(x) = physical_attributes.get(hit.entity) {
                    info!("{:?}", x);
                }
                if let Ok(x) = mental_attributes.get(hit.entity) {
                    info!("{:?}", x);
                }
                if let Ok(x) = values.get(hit.entity) {
                    info!("{:?}", x);
                }
                if let Ok(x) = tasks.get(hit.entity) {
                    info!("{:?}", x);
                }
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
