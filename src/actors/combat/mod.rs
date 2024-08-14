use bevy::prelude::*;

pub mod damage;
pub mod death_effects;
pub mod projectile;

use super::Player;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            projectile::ProjectilePlugin,
            death_effects::DeathEffectsPlugin,
        ))
        .add_event::<AttackEvent>()
        .add_event::<DeathEvent>()
        .add_event::<DamageTakenEvent>()
        .add_systems(PreUpdate, purge_despawned_targets)
        .add_systems(Update, update_aggro_on_player)
        .add_systems(
            PostUpdate,
            (damage::process_attacks, damage::do_death).chain(),
        )
        .register_type::<Damage>();
    }
}

#[derive(Bundle, Clone)]
pub struct CombatantBundle {
    pub combat_info: CombatInfo,
    pub death_info: DeathInfo,
}

impl Default for CombatantBundle {
    fn default() -> Self {
        Self {
            combat_info: CombatInfo::new(10.0, 0.0),
            death_info: DeathInfo::default(),
        }
    }
}

#[derive(Component, Clone)]
pub struct CombatInfo {
    pub curr_health: f32,
    pub max_health: f32,
    pub curr_defense: f32,
    pub base_defense: f32,
    pub knockback_multiplier: f32,
}

impl CombatInfo {
    pub fn new(health: f32, defense: f32) -> Self {
        Self {
            curr_health: health,
            max_health: health,
            curr_defense: defense,
            base_defense: defense,
            knockback_multiplier: 1.0,
        }
    }
}

#[derive(Component, Default, Clone)]
pub struct DeathInfo {
    pub death_type: DeathType,
    //death_message: Option<&str>,
}

#[derive(Default, Copy, Clone)]
pub enum DeathType {
    #[default]
    Default,
    LocalPlayer,
    RemotePlayer,
    Immortal,
}

#[derive(Clone, Copy, Debug, Reflect, Default)]
pub struct Damage {
    pub amount: f32,
}

#[derive(Clone, Copy, Event)]
pub struct AttackEvent {
    pub attacker: Entity,
    pub target: Entity,
    pub damage: Damage,
    pub knockback: Vec3,
}

//entities may be despawned depending on event ordering and death behavior
#[derive(Clone, Copy, Event)]
pub struct DamageTakenEvent {
    pub attacker: Entity,
    pub target: Entity,
    //after reductions
    pub damage_taken: Damage,
    pub knockback_impulse: Vec3,
}

#[derive(Clone, Copy, Event)]
pub struct DeathEvent {
    pub final_blow: AttackEvent,
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
        if !self.pqueue.is_empty() && self.pqueue[self.current_target_idx].1 < priority {
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
                if *p > max {
                    (elem_idx, *p)
                } else {
                    (idx, max)
                }
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
            warn!("AggroedOnPlayer contains entity that's not a player!");
            commands.entity(entity).remove::<AggroedOnPlayer>();
        }
    }
}
