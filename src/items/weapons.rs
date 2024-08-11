use bevy::prelude::*;

use crate::{
    actors::{coin::SpawnCoinEvent, AttackEvent, CombatInfo, CombatantBundle, Damage},
    physics::{
        collision::Aabb,
        movement::Velocity,
        query::{self, RaycastHit},
    },
    world::{BlockPhysics, Level},
};

use super::{ItemSystemSet, SwingEndEvent, SwingItemEvent, UseEndEvent, UseItemEvent};

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
    mut swing_hit_writer: EventWriter<SwingEndEvent>,
    level: Res<Level>,
    physics_query: Query<&BlockPhysics>,
    weapon_query: Query<&MeleeWeaponItem>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
) {
    for SwingItemEvent {
        user,
        inventory_slot,
        stack,
        tf,
    } in attack_item_reader.read()
    {
        if let Ok(weapon) = weapon_query.get(stack.id) {
            if let Some(RaycastHit::Object(hit)) = query::raycast(
                query::Raycast::new(tf.translation, tf.forward(), 10.0),
                &level,
                &physics_query,
                &object_query,
                &[*user],
            ) {
                attack_writer.send(AttackEvent {
                    attacker: *user,
                    target: hit.entity,
                    damage: weapon.damage,
                    knockback: tf.forward() * weapon.knockback,
                });
                swing_hit_writer.send(SwingEndEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    result: super::HitResult::Hit(hit.hit_pos),
                });
                info!("melee hit!");
            } else {
                swing_hit_writer.send(SwingEndEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    result: super::HitResult::Miss,
                });
                info!("melee miss!");
            }
        }
    }
}

pub fn launch_coin(
    mut attack_item_reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseEndEvent>,
    mut writer: EventWriter<SpawnCoinEvent>,
    weapon_query: Query<&CoinLauncherItem>,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf,
    } in attack_item_reader.read()
    {
        if let Ok(weapon) = weapon_query.get(stack.id) {
            writer.send(SpawnCoinEvent {
                location: Transform::from_translation(tf.translation),
                velocity: Velocity(tf.forward() * weapon.speed),
                combat: CombatantBundle {
                    combat_info: CombatInfo::new(1.0, 0.0),
                    ..default()
                },
                owner: *user,
                damage: weapon.damage,
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
