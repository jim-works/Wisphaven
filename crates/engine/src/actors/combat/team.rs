use std::num::NonZeroU32;

use bevy::prelude::*;

use physics::collision::{Aabb, BlockPhysics};
use serde::{Deserialize, Serialize};
use world::{block::BlockCoord, level::Level};

use super::Combatant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Component, Serialize, Deserialize)]
pub struct Team(pub Option<NonZeroU32>);

impl Team {
    pub fn can_hit(self, other: Team) -> bool {
        self.0
            .is_none_or(|my_team| other.0.is_none_or(|other_team| my_team != other_team))
    }

    pub fn is_allied(self, other: Team) -> bool {
        !self.can_hit(other)
    }
}

pub const PLAYER_TEAM: Team = Team(Some(NonZeroU32::new(1).unwrap()));
pub const ENEMY_TEAM: Team = Team(Some(NonZeroU32::new(2).unwrap()));

pub fn get_targets_in_range<'a>(
    my_team: Team,
    query: &'a Query<'a, 'a, (Entity, &'a Combatant, &'a GlobalTransform, &'a Team)>,
    origin: Vec3,
    range: f32,
) -> impl Iterator<Item = (Entity, &'a Combatant, &'a GlobalTransform, &'a Team)> {
    let sqr_dist = range * range;
    query.iter().filter(move |(_, _, gtf, other_team)| {
        my_team.can_hit(**other_team) && gtf.translation().distance_squared(origin) <= sqr_dist
    })
}

pub fn get_colliding_targets<'a, 'b: 'a>(
    my_team: Team,
    query: &'b Query<
        'a,
        'a,
        (
            Entity,
            &'a Combatant,
            &'a GlobalTransform,
            &'a Aabb,
            &'a Team,
        ),
    >,
    origin: Vec3,
    aabb: Aabb,
    my_aabb_scale: f32,
) -> impl Iterator<
    Item = (
        Entity,
        &'b Combatant,
        &'b GlobalTransform,
        &'b Aabb,
        &'b Team,
    ),
> {
    query
        .iter()
        .filter(move |(_, _, gtf, target_aabb, other_team)| {
            my_team.can_hit(**other_team)
                && target_aabb.intersects_aabb(
                    gtf.translation(),
                    aabb.scale(Vec3::ONE * my_aabb_scale),
                    origin,
                )
        })
}

pub fn get_allies_in_range<'a>(
    my_team: Team,
    query: &'a Query<'a, 'a, (Entity, &'a Combatant, &'a GlobalTransform, &'a Team)>,
    origin: Vec3,
    range: f32,
) -> impl Iterator<Item = (Entity, &'a Combatant, &'a GlobalTransform, &'a Team)> {
    let sqr_dist = range * range;
    query.iter().filter(move |(_, _, gtf, other_team)| {
        my_team.is_allied(**other_team) && gtf.translation().distance_squared(origin) <= sqr_dist
    })
}

pub fn get_colliding_allies<'a>(
    my_team: Team,
    query: &'a Query<
        'a,
        'a,
        (
            Entity,
            &'a Combatant,
            &'a GlobalTransform,
            &'a Aabb,
            &'a Team,
        ),
    >,
    origin: Vec3,
    aabb: Aabb,
    my_aabb_scale: f32,
) -> impl Iterator<
    Item = (
        Entity,
        &'a Combatant,
        &'a GlobalTransform,
        &'a Aabb,
        &'a Team,
    ),
> {
    query
        .iter()
        .filter(move |(_, _, gtf, target_aabb, other_team)| {
            my_team.is_allied(**other_team)
                && target_aabb.intersects_aabb(
                    gtf.translation(),
                    aabb.scale(Vec3::ONE * my_aabb_scale),
                    origin,
                )
        })
}

//todo improve this
pub fn test_point(
    my_team: Team,
    point: Vec3,
    level: &Level,
    physics_query: &Query<&BlockPhysics>,
    object_query: &Query<(Entity, &GlobalTransform, &Aabb, &Team)>,
    exclude: &[Entity],
) -> Option<Entity> {
    //test entity
    for (entity, tf, col, other_team) in object_query.iter() {
        if exclude.contains(&entity) || !my_team.can_hit(*other_team) {
            continue;
        }
        if col.intersects_point(tf.translation(), point) {
            //our point intersects an entity
            return Some(entity);
        }
    }
    //test block
    let test_block_coord = BlockCoord::from(point);
    if let Some(block_entity) = level.get_block_entity(test_block_coord) {
        if !exclude.contains(&block_entity) {
            if let Some(collider) = physics_query
                .get(block_entity)
                .ok()
                .and_then(Aabb::from_block)
            {
                if collider.intersects_point(test_block_coord.to_vec3(), point) {
                    //our point intersects the block
                    return Some(block_entity);
                }
            }
        }
    }
    None
}

//todo improve this
pub fn test_box(
    my_team: Team,
    point: Vec3,
    aabb: Aabb,
    object_query: &Query<(Entity, &GlobalTransform, &Aabb, &Team)>,
    exclude: &[Entity],
) -> Option<Entity> {
    //test entity
    for (entity, tf, col, other_team) in object_query.iter() {
        if exclude.contains(&entity) || !my_team.can_hit(*other_team) {
            continue;
        }
        if aabb.intersects_aabb(point, *col, tf.translation()) {
            //our point intersects an entity
            return Some(entity);
        }
    }
    None
}
