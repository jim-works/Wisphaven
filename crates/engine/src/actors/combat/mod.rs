use std::time::Duration;

use bevy::{ecs::entity::EntityHashMap, prelude::*};
use serde::{Deserialize, Serialize};
use team::*;

pub mod damage;
pub mod death_effects;
pub mod projectile;
pub mod team;

use interfaces::scheduling::*;
use physics::collision::Aabb;

use super::Player;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            projectile::ProjectilePlugin,
            death_effects::DeathEffectsPlugin,
            damage::DamagePlugin,
        ))
        .add_event::<AttackEvent>()
        .add_event::<DeathEvent>()
        .add_event::<DamageTakenEvent>()
        .add_systems(Startup, create_level_entity)
        .add_systems(PreUpdate, purge_despawned_targets)
        .add_systems(
            FixedUpdate,
            (update_aggro_on_player, do_contact_damage).in_set(LevelSystemSet::Tick),
        )
        .add_systems(PostUpdate, update_combat_relationships)
        .insert_resource(CombatantRelationships::default())
        .register_type::<Damage>();
    }
}

#[derive(Resource)]
pub struct LevelEntity(Entity);

#[derive(Bundle, Clone, Serialize, Deserialize, Debug)]
pub struct CombatantBundle {
    pub combatant: Combatant,
    pub death_info: DeathInfo,
    pub invulnerability: Invulnerability,
    pub team: Team,
}

#[derive(Component, Clone)]
pub struct ContactDamage {
    pub damage: Damage,
    pub knockback: f32,
}

impl ContactDamage {
    pub fn new(damage: Damage) -> Self {
        Self {
            damage,
            knockback: 0.1,
        }
    }
}

impl Default for CombatantBundle {
    fn default() -> Self {
        Self {
            combatant: Combatant::new(10.0, 0.0),
            death_info: DeathInfo::default(),
            invulnerability: Invulnerability::default(),
            team: PLAYER_TEAM,
        }
    }
}

#[derive(Component, Clone, Serialize, Deserialize, Debug)]
pub struct Invulnerability {
    pub until: Duration,
    pub time_after_hit: Duration,
}

impl Default for Invulnerability {
    fn default() -> Self {
        Self {
            until: Default::default(),
            time_after_hit: Duration::from_secs_f32(0.1),
        }
    }
}

impl Invulnerability {
    pub fn new(time_after_hit: Duration) -> Self {
        Self {
            time_after_hit,
            ..default()
        }
    }
    pub fn on_hit(&mut self, current_time: Duration) {
        self.until = current_time + self.time_after_hit;
    }
    pub fn is_active(&self, current_time: Duration) -> bool {
        self.until >= current_time
    }
}

#[derive(Component, Clone, Serialize, Deserialize, Debug)]
pub enum Combatant {
    Root { health: Health, defense: Defense },
    Child { parent: Entity, defense: Defense },
}

impl Combatant {
    pub fn new(health: f32, defense: f32) -> Self {
        Self::Root {
            health: Health::new(health),
            defense: Defense::new(defense),
        }
    }
    pub fn new_child(parent: Entity, defense: f32) -> Self {
        Self::Child {
            parent,
            defense: Defense::new(defense),
        }
    }

    //returns farthest ancestor (other than me)
    pub fn get_ancestor(&self, query: &Query<&Combatant>) -> Option<Entity> {
        match self {
            Combatant::Root { .. } => None,
            Combatant::Child { parent, .. } => {
                let Ok(parent_combatant) = query.get(*parent) else {
                    return None;
                };
                parent_combatant.get_ancestor(query).or(Some(*parent))
            }
        }
    }

    pub fn has_ancestor(&self, entity: Entity, query: &Query<&Combatant>) -> bool {
        match self {
            Combatant::Root { .. } => false,
            Combatant::Child { parent, .. } => {
                if *parent == entity {
                    return true;
                }
                let Ok(parent_combatant) = query.get(*parent) else {
                    return false;
                };
                parent_combatant.has_ancestor(entity, query)
            }
        }
    }

    pub fn get_root<'a>(
        &'a self,
        query: &'a Query<'a, 'a, &'a Combatant>,
    ) -> Option<&'a Combatant> {
        match self {
            Combatant::Root { .. } => Some(self),
            Combatant::Child { .. } => {
                let root = self
                    .get_ancestor(query)
                    .and_then(|root| query.get(root).ok());
                match root {
                    Some(Combatant::Root { .. }) => root,
                    _ => {
                        warn!("combatant has no root");
                        None
                    }
                }
            }
        }
    }

    pub fn get_health(&self, query: &Query<&Combatant>) -> Option<Health> {
        match self {
            Combatant::Root { health, .. } => Some(*health),
            Combatant::Child { .. } => {
                let root = self
                    .get_ancestor(query)
                    .and_then(|root| query.get(root).ok());
                match root {
                    Some(Combatant::Root { health, .. }) => Some(*health),
                    _ => {
                        warn!("combatant has no root");
                        None
                    }
                }
            }
        }
    }
}

pub trait GetRootMut<'a> {
    fn get_root_mut(
        self,
        query: &'a mut Query<'a, 'a, &'a mut Combatant>,
    ) -> Option<Mut<'a, Combatant>>;
}

impl<'a> GetRootMut<'a> for Mut<'a, Combatant> {
    fn get_root_mut(
        mut self,
        query: &'a mut Query<'a, 'a, &'a mut Combatant>,
    ) -> Option<Mut<'a, Combatant>> {
        match self.as_mut() {
            Combatant::Root { .. } => Some(self),
            Combatant::Child { .. } => {
                let mut lens = query.transmute_lens::<&Combatant>();
                let root = self
                    .get_ancestor(&lens.query())
                    .and_then(|root| query.get_mut(root).ok());
                match root {
                    Some(root_mut) => Some(root_mut),
                    _ => {
                        warn!("combatant has no root");
                        None
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default, Debug, Deserialize, Serialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(amount: f32) -> Self {
        Self {
            current: amount,
            max: amount,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct Defense {
    pub current: f32,
    pub base: f32,
}

impl Defense {
    pub fn new(amount: f32) -> Self {
        Self {
            current: amount,
            base: amount,
        }
    }
}

#[derive(Resource, Default)]
struct CombatantRelationships {
    map: EntityHashMap<Vec<Entity>>,
}

impl CombatantRelationships {
    // may contain entities that don't exist
    fn get_children_unfiltered(&self, parent: Entity) -> Option<impl Iterator<Item = &Entity>> {
        self.map.get(&parent).map(|vec| vec.iter())
    }

    fn insert(&mut self, entity: Entity, combatant: &Combatant) {
        match combatant {
            Combatant::Root { .. } => {
                if !self.map.contains_key(&entity) {
                    self.map.insert(entity, Vec::new());
                }
            }
            Combatant::Child { parent, .. } => match self.map.get_mut(parent) {
                Some(children) => children.push(entity),
                None => {
                    self.map.insert(*parent, vec![entity]);
                }
            },
        }
    }

    // may contain entities that don't exist
    fn remove_parent(&mut self, parent: Entity) -> Option<Vec<Entity>> {
        self.map.remove(&parent)
    }
}

fn update_combat_relationships(
    mut commands: Commands,
    added: Query<(Entity, &Combatant), Added<Combatant>>,
    mut removed: RemovedComponents<Combatant>,
    mut relationships: ResMut<CombatantRelationships>,
) {
    for (entity, combatant) in added.iter() {
        relationships.insert(entity, combatant);
    }
    for entity in removed.read() {
        if let Some(mut children) = relationships.remove_parent(entity) {
            for child in children.drain(..) {
                if let Some(ec) = commands.get_entity(child) {
                    ec.despawn_recursive();
                }
            }
        }
    }
}

#[derive(Component, Default, Clone, Serialize, Deserialize, Debug)]
pub struct DeathInfo {
    pub death_type: DeathType,
    //death_message: Option<&str>,
}

#[derive(Default, Copy, Clone, Serialize, Deserialize, Debug)]
pub enum DeathType {
    #[default]
    Default,
    LocalPlayer,
    RemotePlayer,
    Immortal,
}

#[derive(Default, Copy, Clone, Reflect, Debug, Deserialize, Serialize)]
pub enum DamageType {
    #[default]
    Normal,
    HPRemoval,
}

#[derive(Clone, Copy, Debug, Reflect, Default, Deserialize, Serialize)]
pub struct Damage {
    pub amount: f32,
    pub dtype: DamageType,
}

impl Damage {
    pub fn new(amount: f32) -> Self {
        Self {
            amount,
            ..default()
        }
    }

    //calculates actual HP to remove
    pub fn calc(self, defense: Defense) -> f32 {
        //curve sets damage multiplier between 0 and 2. infinite defense gives multiplier 0, -infinite defense gives multiplier 2
        //0 defense gives multiplier 1
        //TODO: maybe switch to sigmoid, I don't think I want armor to have this amount of diminishing returns.
        const DEFENSE_SCALE: f32 = 0.1;
        match self.dtype {
            DamageType::Normal => {
                (1.0 - (DEFENSE_SCALE * defense.current)
                    / (1.0 + (DEFENSE_SCALE * defense.current).abs()))
                    * self.amount
            }
            DamageType::HPRemoval => self.amount,
        }
    }

    //calculates damage done to root, if root exists
    pub fn calc_recursive(self, target: &Combatant, query: &Query<&Combatant>) -> Option<f32> {
        match target {
            Combatant::Root { defense, .. } => Some(self.calc(*defense)),
            Combatant::Child { parent, defense } => {
                let Ok(parent) = query.get(*parent) else {
                    return None;
                };
                let parents_damage = Damage {
                    amount: self.calc(*defense),
                    dtype: self.dtype,
                };
                parents_damage.calc_recursive(parent, query)
            }
        }
    }
}

#[derive(Clone, Copy, Event)]
pub struct AttackEvent {
    pub attacker: Option<Entity>,
    pub target: Entity,
    pub damage: Damage,
    pub knockback: Vec3,
}

//entities may be despawned depending on event ordering and death behavior
#[derive(Clone, Copy, Event)]
pub struct DamageTakenEvent {
    pub attacker: Option<Entity>,
    pub target: Entity,
    //after reductions
    pub damage: Damage,
    pub knockback_impulse: Vec3,
    pub hit_location: Vec3,
}

#[derive(Clone, Copy, Event)]
pub struct DeathEvent {
    pub final_blow: DamageTakenEvent,
    pub damage_taken: f32,
}

//targets the entity is currently considering, based on a priority queue
//entity's current target at front of the queue
//abilities that change attack target (something like berserker's call from dota)
//would add a new target with high priority, add a marker component for the buff, then remove the entity from AggroTargets when the buff expires.
//modify aggrotargets by sending a message
#[derive(Component, Default)]
pub struct AggroTargets {
    pub pqueue: Vec<(Entity, i32)>, //could switch to proper pqueue, but I think this will be faster since we will usually have a small number of targets
    current_target_idx: usize,
}

impl AggroTargets {
    pub fn new(entity_priorities: Vec<(Entity, i32)>) -> Self {
        let mut aggro = AggroTargets {
            pqueue: entity_priorities,
            current_target_idx: 0,
        };
        aggro.recalculate_target();
        aggro
    }
    pub fn current_target(&self) -> Option<Entity> {
        self.pqueue.get(self.current_target_idx).map(|(t, _)| *t)
    }
    pub fn add_target(&mut self, target: Entity, priority: i32) {
        if !self.pqueue.is_empty()
            && (self.pqueue.len() <= self.current_target_idx
                || self.pqueue[self.current_target_idx].1 < priority)
        {
            //this is now the current target
            self.current_target_idx = self.pqueue.len();
        }
        self.pqueue.push((target, priority));
    }
    pub fn remove_target(&mut self, target: Entity) {
        if let Some(idx) = self
            .pqueue
            .iter()
            .enumerate()
            .find(|(_, (e, _))| *e == target)
            .map(|(i, _)| i)
        {
            self.pqueue.remove(idx);
            if self.current_target_idx == idx {
                //find new max priority target
                self.recalculate_target();
            }
        }
    }
    pub fn recalculate_target(&mut self) {
        self.current_target_idx = self
            .pqueue
            .iter()
            .enumerate()
            .fold((0, i32::MIN), |(idx, max), (elem_idx, (_, p))| {
                if *p > max { (elem_idx, *p) } else { (idx, max) }
            })
            .0;
    }
}

fn purge_despawned_targets(mut query: Query<&mut AggroTargets>, entity_query: Query<Entity>) {
    for mut aggro in query.iter_mut() {
        aggro
            .pqueue
            .retain(|(entity, _)| entity_query.contains(*entity));
    }
}

#[derive(Component)]
pub struct AggroPlayer {
    pub range: f32,
    pub priority: i32,
}

impl Default for AggroPlayer {
    fn default() -> Self {
        Self {
            range: f32::INFINITY,
            priority: 0,
        }
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct AggroedOnPlayer(Entity);

fn update_aggro_on_player(
    player_query: Query<(Entity, &GlobalTransform), With<Player>>,
    mut new_aggro_query: Query<
        (Entity, &GlobalTransform, &AggroPlayer, &mut AggroTargets),
        Without<AggroedOnPlayer>,
    >,
    mut curr_aggro_query: Query<(
        Entity,
        &GlobalTransform,
        &AggroPlayer,
        &mut AggroTargets,
        &AggroedOnPlayer,
    )>,
    mut commands: Commands,
) {
    //add new player aggros
    for (entity, tf, aggro, mut targets) in new_aggro_query.iter_mut() {
        //get closest player
        let (sqr_distance, closest_player) = player_query.iter().fold(
            (f32::MAX, Entity::PLACEHOLDER),
            |(curr_d, curr_player), (player_entity, player_tf)| {
                let d = tf.translation().distance_squared(player_tf.translation());
                if d < curr_d {
                    (d, player_entity)
                } else {
                    (curr_d, curr_player)
                }
            },
        );
        if sqr_distance <= aggro.range * aggro.range {
            //player in range
            targets.add_target(closest_player, aggro.priority);
            commands
                .entity(entity)
                .insert(AggroedOnPlayer(closest_player));
        }
    }
    //remove player aggros if they go out of range
    //intentionally not updating to new closest player to make them chase more
    for (entity, tf, aggro, mut targets, AggroedOnPlayer(player)) in curr_aggro_query.iter_mut() {
        if let Ok((player_entity, player_tf)) = player_query.get(*player) {
            if player_tf.translation().distance_squared(tf.translation())
                > aggro.range * aggro.range
            {
                //player too far, drop aggro
                targets.remove_target(player_entity);
                commands.entity(entity).remove::<AggroedOnPlayer>();
            }
        } else {
            commands.entity(entity).remove::<AggroedOnPlayer>();
        }
    }
}

fn create_level_entity(mut commands: Commands) {
    let entity = commands.spawn_empty().id();
    commands.insert_resource(LevelEntity(entity));
}

fn do_contact_damage(
    attacker_query: Query<(Entity, &ContactDamage, &GlobalTransform, &Aabb, &Team)>,
    target_query: Query<(Entity, &GlobalTransform, &Aabb, &Team)>,
    mut attack_writer: EventWriter<AttackEvent>,
) {
    const AABB_SCALE: Vec3 = Vec3::splat(1.1);
    for (entity, cd, gtf, aabb, attacker_team) in attacker_query.iter() {
        for (target_entity, target_gtf, _, target_team) in
            target_query
                .iter()
                .filter(move |(_, target_gtf, target_aabb, target_team)| {
                    attacker_team.can_hit(**target_team)
                        && target_aabb.intersects_aabb(
                            target_gtf.translation(),
                            aabb.scale(AABB_SCALE),
                            gtf.translation(),
                        )
                })
        {
            //they intersect
            attack_writer.send(AttackEvent {
                attacker: Some(entity),
                target: target_entity,
                damage: cd.damage,
                knockback: (target_gtf.translation() - gtf.translation()) * cd.knockback,
            });
        }
    }
}
