use std::time::Duration;

use crate::{
    physics::{
        collision::{Aabb, CollidingBlocks},
        movement::Velocity,
        query::test_point,
    },
    world::{
        events::{BlockDamageSetEvent, ChunkUpdatedEvent},
        BlockCoord, BlockId, BlockPhysics, Level, LevelSystemSet,
    },
};
use bevy::prelude::*;

use super::{AttackEvent, Damage};

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (test_projectile_hit, update_projectile_lifetime)
                .in_set(LevelSystemSet::Main)
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
fn test_projectile_hit(
    query: Query<(
        Entity,
        &GlobalTransform,
        &Projectile,
        Option<&Velocity>,
        Option<&ProjectileSpawnedInEntity>,
        &CollidingBlocks,
    )>,
    mut attack_writer: EventWriter<AttackEvent>,
    mut commands: Commands,
    level: Res<Level>,
    physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    id_query: Query<&BlockId>,
    mut damage_writer: EventWriter<BlockDamageSetEvent>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
) {
    for (proj_entity, tf, proj, v, opt_in_entity, colliding_blocks) in query.iter() {
        let opt_hit_entity = test_point(
            tf.translation(),
            &level,
            &physics_query,
            &object_query,
            &[proj_entity],
        );
        if opt_hit_entity.is_some() || !colliding_blocks.is_empty() {
            let hit_blocks = colliding_blocks.iter().map(|&(coord, _, _)| coord);
            proj_hit(
                &mut attack_writer,
                &mut commands,
                proj_entity,
                hit_blocks,
                proj,
                v,
                opt_hit_entity,
                opt_in_entity,
                &level,
                &id_query,
                &mut damage_writer,
                &mut update_writer,
            );
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

fn proj_hit(
    writer: &mut EventWriter<AttackEvent>,
    commands: &mut Commands,
    proj_entity: Entity,
    proj_hit_blocks: impl Iterator<Item = BlockCoord>,
    proj: &Projectile,
    v: Option<&Velocity>,
    target: Option<Entity>,
    opt_in_entity: Option<&ProjectileSpawnedInEntity>,
    level: &Level,
    id_query: &Query<&BlockId>,
    damage_writer: &mut EventWriter<BlockDamageSetEvent>,
    update_writer: &mut EventWriter<ChunkUpdatedEvent>,
) {
    if let Some(&ProjectileSpawnedInEntity(ignore)) = opt_in_entity {
        //don't want to hit the entity we spawn in
        if target.map(|t| ignore == t).unwrap_or(false) {
            return;
        }
    }
    if let Some(hit) = target {
        writer.send(AttackEvent {
            attacker: proj.owner,
            target: hit,
            damage: proj.damage,
            knockback: v.map_or(Vec3::ZERO, |v| v.0 * proj.knockback_mult),
        });
    }

    if let Some(ref on_hit) = proj.on_hit {
        on_hit(
            ProjectileHit {
                hit: target,
                projectile: proj_entity,
            },
            commands,
        );
    }
    for block_coord in proj_hit_blocks {
        level.damage_block(
            block_coord,
            proj.terrain_damage,
            Some(proj_entity),
            id_query,
            damage_writer,
            update_writer,
            commands,
        );
    }

    match proj.hit_behavior {
        ProjecileHitBehavior::Despawn => commands.entity(proj_entity).despawn_recursive(),
        ProjecileHitBehavior::None => (),
    }
}
