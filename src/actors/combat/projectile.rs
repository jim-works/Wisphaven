use std::time::Duration;

use crate::world::{
    events::{BlockDamageSetEvent, ChunkUpdatedEvent},
    BlockCoord, BlockId, Level, LevelSystemSet,
};
use bevy::{prelude::*, utils::HashSet};
use bevy_rapier3d::prelude::*;

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

#[derive(Component)]
pub struct Projectile {
    pub owner: Entity,
    pub damage: Damage,
    pub terrain_damage: f32,
    pub knockback_mult: f32,
    pub despawn_time: Duration,
    pub despawn_on_hit: bool,
    //usually want same behavior for both, so one function
    pub on_hit_or_despawn: Option<Box<dyn Fn(ProjectileHit, &mut Commands) + Send + Sync>>,
}

//makrs that a projectile could spawn inside of an entity, and should ignore that entity until it is not inside it.
#[derive(Component)]
pub struct ProjectileSpawnedInEntity(pub Entity);

#[derive(Bundle)]
pub struct ProjectileBundle {
    pub projectile: Projectile,
    pub active_events: ActiveEvents,
    pub inside_entity: ProjectileSpawnedInEntity,
}

impl ProjectileBundle {
    //can set projectile.owner to Entity::PLACEHOLDER if projectile doesn't have one
    pub fn new(projectile: Projectile) -> Self {
        Self {
            inside_entity: ProjectileSpawnedInEntity(projectile.owner),
            projectile,
            active_events: ActiveEvents::COLLISION_EVENTS,
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
            if let Some(action) = &proj.on_hit_or_despawn {
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

fn test_projectile_hit(
    query: Query<(
        Entity,
        &Projectile,
        Option<&Velocity>,
        Option<&ProjectileSpawnedInEntity>,
    )>,
    mut collision_events: EventReader<bevy_rapier3d::pipeline::CollisionEvent>,
    mut attack_writer: EventWriter<AttackEvent>,
    mut commands: Commands,
    level: Res<Level>,
    id_query: Query<&BlockId>,
    mut damage_writer: EventWriter<BlockDamageSetEvent>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    rapier_context: Res<RapierContext>,
) {
    for event in collision_events.iter() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            //typically don't get duplicated events, so have to check both entities
            if let Ok((proj_entity, proj, v, opt_in_entity)) = query.get(*e1) {
                proj_hit(
                    &mut attack_writer,
                    &mut commands,
                    proj_entity,
                    proj,
                    v,
                    *e2,
                    opt_in_entity,
                    &level,
                    &id_query,
                    &mut damage_writer,
                    &mut update_writer,
                    &rapier_context,
                );
            } else if let Ok((proj_entity, proj, v, opt_in_entity)) = query.get(*e2) {
                proj_hit(
                    &mut attack_writer,
                    &mut commands,
                    proj_entity,
                    proj,
                    v,
                    *e1,
                    opt_in_entity,
                    &level,
                    &id_query,
                    &mut damage_writer,
                    &mut update_writer,
                    &rapier_context,
                );
            }
        }
    }
}

fn proj_hit(
    writer: &mut EventWriter<AttackEvent>,
    commands: &mut Commands,
    proj_entity: Entity,
    proj: &Projectile,
    v: Option<&Velocity>,
    target: Entity,
    opt_in_entity: Option<&ProjectileSpawnedInEntity>,
    level: &Level,
    id_query: &Query<&BlockId>,
    damage_writer: &mut EventWriter<BlockDamageSetEvent>,
    update_writer: &mut EventWriter<ChunkUpdatedEvent>,
    rapier_context: &RapierContext,
) {
    const PENETRATION_DEPTH: f32 = 0.05; //amount to follow the normal in a collision to move inside the block when calculating damage
    const EPSILON: f32 = 0.5; //ignore multiple hits that have positions and normals within epsilon units of each other
    if let Some(&ProjectileSpawnedInEntity(ignore)) = opt_in_entity {
        //don't want to hit the entity we spawn in
        if ignore == target {
            //we will only start the collision once, so we can remove the component
            commands
                .entity(proj_entity)
                .remove::<ProjectileSpawnedInEntity>();
            return;
        }
    }
    writer.send(AttackEvent {
        attacker: proj.owner,
        target,
        damage: proj.damage,
        knockback: v.map_or_else(|| Vec3::ZERO, |v| v.linvel * proj.knockback_mult),
    });
    if let Some(ref on_hit) = proj.on_hit_or_despawn {
        on_hit(
            ProjectileHit {
                hit: Some(target),
                projectile: proj_entity,
            },
            commands,
        );
    }
    let mut hits = HashSet::new();
    for contact_pair in rapier_context.contacts_with(proj_entity) {
        let invert = contact_pair.collider1() != proj_entity;
        for manifold in contact_pair.manifolds() {
            let normal = manifold.normal();
            for solver_contact in manifold.solver_contacts() {
                let block_coord = BlockCoord::from(
                    solver_contact.point()
                        + if invert { -1.0 } else { 1.0 } * PENETRATION_DEPTH * normal,
                );
                if hits.contains(&block_coord) {
                    continue;
                }
                hits.insert(block_coord);
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
        }
    }

    if proj.despawn_on_hit {
        commands.entity(proj_entity).despawn_recursive();
    }
}
