use std::sync::Arc;

use bevy::prelude::*;

use engine::actors::{AttackEvent, Combatant, CombatantBundle, Damage, team::PLAYER_TEAM};
use interfaces::scheduling::ItemSystemSet;
use physics::{
    collision::{Aabb, BlockPhysics},
    movement::Velocity,
    query::{self, RaycastHit},
};
use world::level::Level;

use actors::{
    coin::SpawnCoin,
    spawning::{
        ProjectileName, ProjectileSpawnArgs, SpawnNamedProjectileEvent, SpawnProjectileEvent,
    },
};

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
    pub name: ProjectileName,
    pub json: Option<String>,
    pub damage: Damage,
    pub speed: f32,
    pub lifetime_mult: f32,
    pub knockback_mult: f32,
    pub terrain_damage_mult: f32,
    cached_name: Option<Arc<ProjectileName>>,
    cached_json: Option<Arc<str>>,
}

impl Default for ProjectileLauncherItem {
    fn default() -> Self {
        Self {
            name: Default::default(),
            json: None,
            damage: Default::default(),
            speed: Default::default(),
            lifetime_mult: 1.,
            knockback_mult: 1.,
            terrain_damage_mult: 1.,
            cached_name: None,
            cached_json: None,
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
                    attacker: Some(*user),
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
    mut writer: EventWriter<SpawnNamedProjectileEvent>,
    mut weapon_query: Query<&mut ProjectileLauncherItem>,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf,
    } in attack_item_reader.read()
    {
        if let Ok(mut weapon) = weapon_query.get_mut(stack.id) {
            weapon.cached_name = Some(
                weapon
                    .cached_name
                    .clone()
                    .unwrap_or(Arc::new(weapon.name.clone())),
            );
            weapon.cached_json = weapon
                .json
                .as_ref()
                .map(|json_str| Arc::from(json_str.as_str()));
            writer.send(SpawnNamedProjectileEvent {
                name: weapon.cached_name.clone().unwrap(),
                spawn_args: ProjectileSpawnArgs {
                    transform: Transform::from_translation(tf.translation),
                    velocity: Velocity(tf.forward() * weapon.speed),
                    combat: CombatantBundle {
                        combatant: Combatant::new(100.0, 0.0),
                        team: PLAYER_TEAM,
                        ..default()
                    },
                    owner: Some(*user),
                    damage: weapon.damage,
                    lifetime: weapon.lifetime_mult,
                    knockback: weapon.knockback_mult,
                    terrain_damage: weapon.terrain_damage_mult,
                },
                json_args: weapon.cached_json.clone(),
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
