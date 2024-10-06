use bevy::prelude::*;

use engine::physics::{grapple::ShootGrappleEvent, query::Raycast};

use engine::items::{HitResult, ItemSystemSet, UseEndEvent, UseItemEvent};

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

#[derive(Component, Reflect)]
#[reflect(Component, FromWorld)]
pub struct GrappleItem {
    pub length: f32,
    pub strength: f32,
    pub max_speed: f32,
    pub remove_distance: Option<f32>,
}

impl Default for GrappleItem {
    fn default() -> Self {
        Self {
            length: 50.,
            strength: 0.5,
            max_speed: 2.,
            remove_distance: Some(2.0),
        }
    }
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
                max_speed: item.max_speed,
            });
            hit_writer.send(UseEndEvent {
                user: *user,
                inventory_slot: *inventory_slot,
                stack: *stack,
                result: HitResult::Miss,
            });
        }
    }
}
