use bevy::{ecs::query::QueryFilter, prelude::*};

use crate::physics::collision::Aabb;

use super::Combatant;

#[derive(Component, Clone, Copy, Default)]
pub struct PlayerTeam;

#[derive(Component, Clone, Copy, Default)]
pub struct EnemyTeam;

#[derive(Component, Clone, Copy, Default)]
pub struct FreeForAllTeam;

//when adding a new team, add to macro below
pub trait Team: Component + Clone + Copy + Default + Send + Sync {
    type Targets: QueryFilter;
    type Allies: QueryFilter;
}

impl Team for PlayerTeam {
    type Targets = Or<(With<EnemyTeam>, With<FreeForAllTeam>)>;
    type Allies = With<PlayerTeam>;
}
impl Team for EnemyTeam {
    type Targets = Or<(With<PlayerTeam>, With<FreeForAllTeam>)>;
    type Allies = With<EnemyTeam>;
}
impl Team for FreeForAllTeam {
    type Targets = Or<(With<PlayerTeam>, With<EnemyTeam>, With<FreeForAllTeam>)>;
    type Allies = ();
}

//for use when building app, for example:
//  app.add_systems(FixedUpdate, all_teams_system!(do_contact_damage))
//will expand the macro to a tuple with do_contact_damage::<Team> for all teams
#[macro_export]
macro_rules! all_teams_system {
    (
        $name:ident
    ) => {
        (
            $name::<PlayerTeam>,
            $name::<EnemyTeam>,
            $name::<FreeForAllTeam>,
        )
    };
}

pub trait AddTeam {
    type TargetResultType;
    type AllyResultType;

    fn append_target<T>(self, t: T) -> Self::TargetResultType;
    fn append_ally<T>(self, t: T) -> Self::AllyResultType;
}

pub fn get_targets_in_range<'a, T: Team>(
    query: &'a Query<'a, 'a, (Entity, &'a Combatant, &'a GlobalTransform), T::Targets>,
    origin: Vec3,
    range: f32,
) -> impl Iterator<Item = (Entity, &'a Combatant, &'a GlobalTransform)> {
    let sqr_dist = range * range;
    query
        .iter()
        .filter(move |(_, _, gtf)| gtf.translation().distance_squared(origin) <= sqr_dist)
}

pub fn get_colliding_targets<'a, 'b: 'a, T: Team>(
    query: &'b Query<'a, 'a, (Entity, &'a Combatant, &'a GlobalTransform, &'a Aabb), T::Targets>,
    origin: Vec3,
    aabb: Aabb,
    my_aabb_scale: f32,
) -> impl Iterator<Item = (Entity, &'b Combatant, &'b GlobalTransform, &'b Aabb)> {
    query.iter().filter(move |(_, _, gtf, target_aabb)| {
        target_aabb.intersects_aabb(
            gtf.translation(),
            aabb.scale(Vec3::ONE * my_aabb_scale),
            origin,
        )
    })
}

pub fn get_allies_in_range<'a, T: Team>(
    query: &'a Query<'a, 'a, (Entity, &'a Combatant, &'a GlobalTransform), T::Allies>,
    origin: Vec3,
    range: f32,
) -> impl Iterator<Item = (Entity, &'a Combatant, &'a GlobalTransform)> {
    let sqr_dist = range * range;
    query
        .iter()
        .filter(move |(_, _, gtf)| gtf.translation().distance_squared(origin) <= sqr_dist)
}

pub fn get_colliding_allies<'a, T: Team>(
    query: &'a Query<'a, 'a, (Entity, &'a Combatant, &'a GlobalTransform, &'a Aabb), T::Allies>,
    origin: Vec3,
    aabb: Aabb,
    my_aabb_scale: f32,
) -> impl Iterator<Item = (Entity, &'a Combatant, &'a GlobalTransform, &'a Aabb)> {
    query.iter().filter(move |(_, _, gtf, target_aabb)| {
        target_aabb.intersects_aabb(
            gtf.translation(),
            aabb.scale(Vec3::ONE * my_aabb_scale),
            origin,
        )
    })
}
