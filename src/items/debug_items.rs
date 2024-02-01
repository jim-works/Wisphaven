use bevy::prelude::*;

use crate::{
    actors::personality::components::*,
    physics::{query::{raycast, Ray, RaycastHit}, collision::Aabb}, world::{BlockPhysics, Level},
};

use super::{ItemSystemSet, UseItemEvent};

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
#[reflect(Component)]
pub struct PersonalityTester;

pub fn use_personality_item(
    mut reader: EventReader<UseItemEvent>,
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
        inventory_slot: _,
        stack,
        tf,
    } in reader.read()
    {
        if personality_item.contains(stack.id) {
            if let Some(RaycastHit::Object(hit)) = raycast(
                Ray::new(tf.translation, tf.forward(), 10.0),
                &level,
                &physics_query,
                &object_query,
                vec![*user],
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
            } else {
                info!("No entity hit!");
            }
        }
    }
}
