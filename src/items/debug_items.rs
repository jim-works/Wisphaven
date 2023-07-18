use bevy::prelude::*;
use bevy_rapier3d::prelude::{QueryFilter, RapierContext};

use crate::{actors::personality::components::*, world::LevelSystemSet};

use super::UseItemEvent;

pub struct DebugItems;

impl Plugin for DebugItems {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, use_personality_item.in_set(LevelSystemSet::Main))
            .register_type::<PersonalityTester>()
        ;
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct PersonalityTester;

pub fn use_personality_item(
    mut reader: EventReader<UseItemEvent>,
    tf_query: Query<&GlobalTransform>,
    physical_attributes: Query<&PhysicalAttributes>,
    mental_attributes: Query<&MentalAttributes>,
    values: Query<&PersonalityValues>,
    tasks: Query<&TaskSet>,
    personality_item: Query<&PersonalityTester>,
    physics: Res<RapierContext>,
) {
    for event in reader.iter() {
        if personality_item.get(event.1.id).is_ok() {
            if let Ok(tf) = tf_query.get(event.0) {
                let groups = QueryFilter::default().exclude_collider(event.0);
                if let Some((hit, _)) =
                    physics.cast_ray(tf.translation(), tf.forward(), 10.0, true, groups)
                {
                    if let Ok(x) = physical_attributes.get(hit) {
                        info!("{:?}", x);
                    }
                    if let Ok(x) = mental_attributes.get(hit) {
                        info!("{:?}", x);
                    }
                    if let Ok(x) = values.get(hit) {
                        info!("{:?}", x);
                    }
                    if let Ok(x) = tasks.get(hit) {
                        info!("{:?}", x);
                    }
                } else {
                    info!("No entity hit!");
                }
            } else {
                info!("Using entity has no transform!");
            }
        }
    }
}
