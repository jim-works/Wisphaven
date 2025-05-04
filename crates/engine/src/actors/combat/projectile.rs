use std::time::Duration;

use bevy::prelude::*;
use interfaces::scheduling::*;
use physics::{
    collision::{Aabb, CollidingBlocks},
    movement::Velocity,
};
use rand::thread_rng;
use world::events::DealBlockDamageEvent;

use crate::items::{
    SpawnDroppedItemEvent,
    loot::{CachedLootTable, ItemLootTable},
};

use super::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (test_projectile_hit, update_projectile_lifetime)
                .in_set(LevelSystemSet::PreTick)
                .chain(),
        );
    }
}

// use observer, triggered when projectile lifetime ends or hits something
#[derive(Event)]
pub struct ProjectileHit {
    hit: Option<Entity>,
}

#[derive(Clone, Copy, Default)]
pub enum ProjecileHitBehavior {
    #[default]
    // also drops items if the projectile has a `ItemLootTable` component
    Despawn,
    None,
}

#[derive(Component, Clone)]
pub struct Projectile {
    pub owner: Option<Entity>,
    pub damage: Damage,
    pub terrain_damage: f32,
    pub knockback_mult: f32,
    pub despawn_time: Duration,
    pub hit_behavior: ProjecileHitBehavior,
}

impl Default for Projectile {
    fn default() -> Self {
        Self {
            owner: None,
            damage: default(),
            terrain_damage: 0.,
            knockback_mult: 0.,
            despawn_time: Duration::from_secs(5),
            hit_behavior: default(),
        }
    }
}

//makrs that a projectile could spawn inside of an entity, and should ignore that entity until it is not inside it.
#[derive(Component)]
#[require(Projectile)]
pub struct ProjectileSpawnedInEntity(pub Entity);

fn update_projectile_lifetime(
    query: Query<(
        Entity,
        &Projectile,
        &GlobalTransform,
        Option<&CachedLootTable<Entity>>,
        Option<&ItemLootTable>,
    )>,
    mut commands: Commands,
    mut drop_writer: EventWriter<SpawnDroppedItemEvent>,
    time: Res<Time>,
) {
    let curr_time = time.elapsed();
    let mut rng = thread_rng();
    for (entity, proj, gtf, opt_cached_loot, opt_loot) in query.iter() {
        if proj.despawn_time < curr_time {
            commands.trigger_targets(ProjectileHit { hit: None }, entity);
            commands.entity(entity).despawn_recursive();
            // make sure to also update the on hit behavior
            if opt_loot.is_some()
                && let Some(cached_loot) = opt_cached_loot
            {
                cached_loot.drop_items(gtf.translation(), &mut drop_writer, &mut rng);
            }
        }
    }
}

//todo - add collision events
fn test_projectile_hit(
    query: Query<(
        Entity,
        &GlobalTransform,
        &Projectile,
        Option<&Velocity>,
        Option<&ProjectileSpawnedInEntity>,
        Option<&CachedLootTable<Entity>>,
        Option<&ItemLootTable>,
        &CollidingBlocks,
        &Aabb,
        &Team,
    )>,
    mut attack_writer: EventWriter<AttackEvent>,
    mut commands: Commands,
    object_query: Query<(Entity, &GlobalTransform, &Aabb, &Team)>,
    mut damage_writer: EventWriter<DealBlockDamageEvent>,
    mut drop_writer: EventWriter<SpawnDroppedItemEvent>,
) {
    let mut rng = thread_rng();
    for (
        proj_entity,
        tf,
        proj,
        v,
        opt_in_entity,
        opt_cached_loot,
        opt_loot,
        colliding_blocks,
        aabb,
        my_team,
    ) in query.iter()
    {
        let opt_hit_entity = test_box(
            *my_team,
            tf.translation(),
            *aabb,
            &object_query,
            &[proj_entity],
        );
        if opt_hit_entity.is_some() || !colliding_blocks.is_empty() {
            let hit_blocks = colliding_blocks.iter().map(|&(coord, _, _)| coord);
            if let Some(&ProjectileSpawnedInEntity(ignore)) = opt_in_entity {
                //don't want to hit the entity we spawn in
                if opt_hit_entity.map(|t| ignore == t).unwrap_or(false) {
                    return;
                }
            }
            if let Some(hit) = opt_hit_entity {
                attack_writer.send(AttackEvent {
                    attacker: proj.owner,
                    target: hit,
                    damage: proj.damage,
                    knockback: v.map_or(Vec3::ZERO, |v| v.0 * proj.knockback_mult),
                });
            }
            commands.trigger_targets(
                ProjectileHit {
                    hit: opt_hit_entity,
                },
                proj_entity,
            );
            for block_position in hit_blocks {
                damage_writer.send(DealBlockDamageEvent {
                    block_position,
                    damage: proj.terrain_damage,
                    damager: Some(proj_entity),
                });
            }

            // make sure to also update the case where the projectile times out
            match proj.hit_behavior {
                ProjecileHitBehavior::Despawn => {
                    if opt_loot.is_some()
                        && let Some(cached_loot) = opt_cached_loot
                    {
                        cached_loot.drop_items(tf.translation(), &mut drop_writer, &mut rng);
                    }
                    commands.entity(proj_entity).despawn_recursive()
                }
                ProjecileHitBehavior::None => (),
            }
        } else if opt_hit_entity.is_none() {
            //can remove spawned in entity since we are outside of all entities
            if opt_in_entity.is_some() {
                commands
                    .entity(proj_entity)
                    .remove::<ProjectileSpawnedInEntity>();
            }
        }
    }
}
