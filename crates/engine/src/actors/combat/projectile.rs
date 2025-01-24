use std::time::Duration;

use crate::{
    all_teams_system,
    physics::{
        collision::{Aabb, CollidingBlocks},
        movement::Velocity,
        query::test_box,
    },
    world::{events::DealBlockDamageEvent, LevelSystemSet},
};
use bevy::prelude::*;

use super::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                all_teams_system!(test_projectile_hit),
                update_projectile_lifetime,
            )
                .in_set(LevelSystemSet::PreTick)
                .chain(),
        );
    }
}

pub struct ProjectileHit {
    hit: Option<Entity>,
    projectile: Entity,
}

#[derive(Clone, Copy, Default)]
pub enum ProjecileHitBehavior {
    #[default]
    Despawn,
    None,
}

#[derive(Component)]
pub struct Projectile {
    pub owner: Entity,
    pub damage: Damage,
    pub terrain_damage: f32,
    pub knockback_mult: f32,
    pub despawn_time: Duration,
    pub hit_behavior: ProjecileHitBehavior,
    //usually want same behavior for both, so one function
    pub on_hit: Option<Box<dyn Fn(ProjectileHit, &mut Commands) + Send + Sync>>,
}

//makrs that a projectile could spawn inside of an entity, and should ignore that entity until it is not inside it.
#[derive(Component)]
pub struct ProjectileSpawnedInEntity(pub Entity);

#[derive(Bundle)]
pub struct ProjectileBundle {
    pub projectile: Projectile,
    pub inside_entity: ProjectileSpawnedInEntity,
}

impl ProjectileBundle {
    //can set projectile.owner to Entity::PLACEHOLDER if projectile doesn't have one
    pub fn new(projectile: Projectile) -> Self {
        Self {
            inside_entity: ProjectileSpawnedInEntity(projectile.owner),
            projectile,
        }
    }
}

fn update_projectile_lifetime(
    query: Query<(Entity, &Projectile)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let curr_time = time.elapsed();
    for (entity, proj) in query.iter() {
        if proj.despawn_time < curr_time {
            if let Some(action) = &proj.on_hit {
                action(
                    ProjectileHit {
                        hit: None,
                        projectile: entity,
                    },
                    &mut commands,
                );
            }
            commands.entity(entity).despawn_recursive();
        }
    }
}

//todo - add collision events
fn test_projectile_hit<T: Team>(
    query: Query<
        (
            Entity,
            &GlobalTransform,
            &Projectile,
            Option<&Velocity>,
            Option<&ProjectileSpawnedInEntity>,
            &CollidingBlocks,
            &Aabb,
        ),
        With<T>,
    >,
    mut attack_writer: EventWriter<AttackEvent>,
    mut commands: Commands,
    object_query: Query<(Entity, &GlobalTransform, &Aabb), T::Targets>,
    mut damage_writer: EventWriter<DealBlockDamageEvent>,
) {
    for (proj_entity, tf, proj, v, opt_in_entity, colliding_blocks, aabb) in query.iter() {
        let opt_hit_entity = test_box::<T>(tf.translation(), *aabb, &object_query, &[proj_entity]);
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

            if let Some(ref on_hit) = proj.on_hit {
                on_hit(
                    ProjectileHit {
                        hit: opt_hit_entity,
                        projectile: proj_entity,
                    },
                    &mut commands,
                );
            }
            for block_position in hit_blocks {
                damage_writer.send(DealBlockDamageEvent {
                    block_position,
                    damage: proj.terrain_damage,
                    damager: Some(proj_entity),
                });
            }

            match proj.hit_behavior {
                ProjecileHitBehavior::Despawn => commands.entity(proj_entity).despawn_recursive(),
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
