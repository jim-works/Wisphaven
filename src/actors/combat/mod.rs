use bevy::prelude::*;

mod damage;
pub mod projectile;
pub use damage::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(projectile::ProjectilePlugin)
            .add_event::<AttackEvent>()
            .add_event::<DeathEvent>()
            .add_systems(PreUpdate, purge_despawned_targets)
            .add_systems(PostUpdate, (process_attacks, do_death).chain())
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
            current_target_idx: 0
        };
        aggro.recalculate_target();
        aggro
    }
    pub fn current_target(&self) -> Option<Entity> {
        self.pqueue.get(self.current_target_idx).map(|(t, _)| *t)
    }
    pub fn add_target(&mut self, target: Entity, priority: i32) {
        if self.pqueue[self.current_target_idx].1 < priority {
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
        self.current_target_idx = self.pqueue.iter().enumerate().fold(
            (0, i32::MIN),
            |(idx, max), (elem_idx, (_, p))| {
                if *p > max {
                    (elem_idx, *p)
                } else {
                    (idx, max)
                }
            },
        ).0;
    }
}

fn purge_despawned_targets(mut query: Query<&mut AggroTargets>, entity_query: Query<Entity>) {
    for mut aggro in query.iter_mut() {
        aggro.pqueue.retain(|(entity, _)| entity_query.contains(*entity));
    }
}
