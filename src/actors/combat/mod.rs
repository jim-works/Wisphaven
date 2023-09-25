use bevy::prelude::*;

mod damage;
pub mod projectile;
pub use damage::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(projectile::ProjectilePlugin)
            .add_event::<AttackEvent>()
            .add_event::<DeathEvent>()
            .add_systems(PostUpdate, (process_attacks, do_death).chain())
            .register_type::<Damage>()
        ;
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
            combat_info: CombatInfo::new(10.0,0.0),
            death_info: DeathInfo::default()
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
    #[default] Default,
    LocalPlayer,
    RemotePlayer,
    Immortal,
}

#[derive(Clone, Copy, Debug, Reflect, Default)]
pub struct Damage {
    pub amount: f32
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

//targets the entity is currently considering, based on a stack
//entity's current target is the top of the stack
//abilities that change attack target (something like berserker's call from dota)
//would push a new target onto the vec, add a marker component for the buff, then remove the entity from AggroTargets when the buff expires.
#[derive(Component, Default, Deref, DerefMut)]
pub struct AggroTargets(pub Vec<Entity>);