use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::actors::AttackEvent;

use super::SwingItemEvent;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct MeleeWeaponItem {
    pub damage: f32,
    pub knockback: f32,
}

pub fn attack_melee (
    mut attack_item_reader: EventReader<SwingItemEvent>,
    mut attack_writer: EventWriter<AttackEvent>,
    collision: Res<RapierContext>,
    weapon_query: Query<&MeleeWeaponItem>
) {
    for item_event in attack_item_reader.iter() {
        if let Ok(weapon) = weapon_query.get(item_event.1.id) {
            let groups = QueryFilter {
                groups: Some(CollisionGroups::new(
                    Group::ALL,
                    Group::from_bits_truncate(crate::physics::ACTOR_GROUP),
                )),
                ..default()
            }.exclude_collider(item_event.0);
            if let Some((hit,_)) = collision.cast_ray(item_event.2.translation(), item_event.2.forward(), 10.0, true, groups) {
                attack_writer.send(AttackEvent { attacker: item_event.0, target: hit, damage: weapon.damage, knockback: item_event.2.forward()*weapon.knockback })
            }
        }
    }
}