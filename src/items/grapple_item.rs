use bevy::prelude::*;

use crate::physics::{grapple::ShootGrappleEvent, query::Raycast};

use super::{ItemSystemSet, UseEndEvent, UseItemEvent};

pub struct GrappleItemPlugin;

impl Plugin for GrappleItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            launch_grapple.in_set(ItemSystemSet::UsageProcessing),
        );
        app.register_type::<GrappleItem>();
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct GrappleItem {
    length: f32,
    strength: f32,
    remove_distance: Option<f32>,
}

pub fn launch_grapple(
    mut attack_item_reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseEndEvent>,
    mut writer: EventWriter<ShootGrappleEvent>,
    item_query: Query<&GrappleItem>,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf,
    } in attack_item_reader.read()
    {
        if let Ok(item) = item_query.get(stack.id) {
            writer.send(ShootGrappleEvent {
                owner: *user,
                ray: Raycast::new(tf.translation, tf.forward(), item.length),
                strength: item.strength,
                remove_distance: item.remove_distance,
            });
            hit_writer.send(UseEndEvent {
                user: *user,
                inventory_slot: *inventory_slot,
                stack: *stack,
                result: super::HitResult::Miss,
            })
        }
    }
}
