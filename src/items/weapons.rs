use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::actors::{CombatInfo, AttackEvent};

use super::{EquipItemEvent, UnequipItemEvent, AttackItemEvent, ItemRegistry};

#[derive(Component)]
pub struct MeleeWeaponItem {
    damage: f32,
    knockback: f32,
}

pub fn equip_unequip_weapon (
    mut equip_reader: EventReader<EquipItemEvent>,
    mut unequip_reader: EventReader<UnequipItemEvent>,
    mut combat_query: Query<&mut CombatInfo>
) {
    for event in unequip_reader.iter() {
        if let Ok(mut info) = combat_query.get_mut(event.0) {
            if info.equipped_weapon == Some(event.1.clone()) {
                info.equipped_weapon = None;
            }
        }
    }
    for event in equip_reader.iter() {
        if let Ok(mut info) = combat_query.get_mut(event.0) {
            info.equipped_weapon = Some(event.1.clone());
        }
    }
}

pub fn attack_melee (
    mut attack_item_reader: EventReader<AttackItemEvent>,
    mut attack_writer: EventWriter<AttackEvent>,
    collision: Res<RapierContext>,
    registry: Res<ItemRegistry>,
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