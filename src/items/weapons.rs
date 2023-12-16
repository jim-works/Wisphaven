use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use itertools::Either::Left;

use crate::{
    actors::{coin::SpawnCoinEvent, AttackEvent, CombatInfo, CombatantBundle, Damage},
    physics::raycast::{self, RaycastHit},
    world::{BlockPhysics, Level},
};

use super::{ItemSystemSet, SwingItemEvent, UseItemEvent};

pub struct WeaponItemPlugin;

impl Plugin for WeaponItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attack_melee, launch_coin).in_set(ItemSystemSet::UsageProcessing),
        )
        .register_type::<CoinLauncherItem>();
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct MeleeWeaponItem {
    pub damage: Damage,
    pub knockback: f32,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct CoinLauncherItem {
    pub damage: Damage,
    pub speed: f32,
}

pub fn attack_melee(
    mut attack_item_reader: EventReader<SwingItemEvent>,
    mut attack_writer: EventWriter<AttackEvent>,
    level: Res<Level>,
    physics_query: Query<&BlockPhysics>,
    weapon_query: Query<&MeleeWeaponItem>,
) {
    for SwingItemEvent {
        user,
        inventory_slot: _,
        stack,
        tf,
    } in attack_item_reader.read()
    {
        if let Ok(weapon) = weapon_query.get(stack.id) {
            let groups = QueryFilter {
                groups: Some(CollisionGroups::new(
                    Group::ALL,
                    Group::from_bits_truncate(crate::physics::ACTOR_GROUP),
                )),
                ..default()
            }
            .exclude_collider(*user);
            if let Some(RaycastHit::Object(_, hit)) = raycast::raycast(
                raycast::Ray::new(tf.translation(), tf.forward(), 10.0),
                &level,
                &physics_query,
            ) {
                attack_writer.send(AttackEvent {
                    attacker: *user,
                    target: hit,
                    damage: weapon.damage,
                    knockback: tf.forward() * weapon.knockback,
                })
            }
        }
    }
}

pub fn launch_coin(
    mut attack_item_reader: EventReader<UseItemEvent>,
    mut writer: EventWriter<SpawnCoinEvent>,
    weapon_query: Query<&CoinLauncherItem>,
) {
    for UseItemEvent {
        user,
        inventory_slot: _,
        stack,
        tf,
    } in attack_item_reader.read()
    {
        if let Ok(weapon) = weapon_query.get(stack.id) {
            writer.send(SpawnCoinEvent {
                location: Transform::from_translation(tf.translation()),
                velocity: tf.forward() * weapon.speed,
                combat: CombatantBundle {
                    combat_info: CombatInfo::new(1.0, 0.0),
                    ..default()
                },
                owner: *user,
                damage: weapon.damage,
            });
        }
    }
}
