use std::sync::Arc;

use bevy::prelude::*;

use engine::actors::{team::PlayerTeam, AttackEvent, Combatant, CombatantBundle, Damage};
use interfaces::scheduling::ItemSystemSet;
use physics::{
    collision::{Aabb, BlockPhysics},
    movement::Velocity,
    query::{self, RaycastHit},
};
use world::level::Level;

use actors::spawning::{ProjectileSpawnArgs, SpawnProjectileEvent};

use engine::items::{HitResult, SwingEndEvent, SwingItemEvent, UseEndEvent, UseItemEvent};

pub struct WeaponItemPlugin;

impl Plugin for WeaponItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attack_melee, launch_coin).in_set(ItemSystemSet::UsageProcessing),
        )
        .register_type::<ProjectileLauncherItem>()
        .register_type::<MeleeWeaponItem>();
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, FromWorld)]
pub struct MeleeWeaponItem {
    pub damage: Damage,
    pub knockback: f32,
}

#[derive(Component, Reflect)]
#[reflect(Component, FromWorld)]
pub struct ProjectileLauncherItem {
    pub name: String,
    pub damage: Damage,
    pub speed: f32,
    pub lifetime_mult: f32,
    pub knockback_mult: f32,
    pub terrain_damage_mult: f32,
}

impl Default for ProjectileLauncherItem {
    fn default() -> Self {
        Self {
            name: Default::default(),
            damage: Default::default(),
            speed: Default::default(),
            lifetime_mult: 1.,
            knockback_mult: 1.,
            terrain_damage_mult: 1.,
        }
    }
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
                    result: HitResult::Hit(hit.hit_pos),
                });
                info!("melee hit!");
            } else {
                swing_hit_writer.send(SwingEndEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    result: HitResult::Miss,
                });
                info!("melee miss!");
            }
        }
    }
}

pub fn launch_coin(
    mut attack_item_reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseEndEvent>,
    mut writer: EventWriter<SpawnProjectileEvent<PlayerTeam>>,
    weapon_query: Query<&ProjectileLauncherItem>,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf,
    } in attack_item_reader.read()
    {
        if let Ok(weapon) = weapon_query.get(stack.id) {
            writer.send(SpawnProjectileEvent {
                name: Arc::new(weapon.name.clone()),
                default: actors::spawning::DefaultSpawnArgs {
                    transform: Transform::from_translation(tf.translation),
                },
                projectile: ProjectileSpawnArgs::<PlayerTeam> {
                    velocity: Velocity(tf.forward() * weapon.speed),
                    combat: CombatantBundle {
                        combatant: Combatant::new(100.0, 0.0),
                        ..default()
                    },
                    owner: *user,
                    damage: weapon.damage,
                    lifetime_mult: weapon.lifetime_mult,
                    knockback_mult: weapon.knockback_mult,
                    terrain_damage_mult: weapon.terrain_damage_mult,
                },
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
