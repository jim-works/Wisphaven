use bevy::prelude::*;

mod damage;
pub use damage::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AttackEvent>()
            .add_event::<DeathEvent>()
            .add_systems(PostUpdate, (process_attacks, do_death).chain())
        ;
    }
}

#[derive(Bundle)]
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

#[derive(Component)]
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
            knockback_multiplier: 0.0,
        }
    }
}

#[derive(Component)]
#[derive(Default)]
pub struct DeathInfo {
    pub death_type: DeathType,
    //death_message: Option<&str>,
}



#[derive(Default)]
pub enum DeathType {
    #[default] Default,
    LocalPlayer,
    RemotePlayer,
    Immortal,
}

#[derive(Clone, Copy, Event)]
pub struct AttackEvent {
    pub attacker: Entity,
    pub target: Entity,
    pub damage: f32,
    pub knockback: Vec3,
}

#[derive(Clone, Copy, Event)]
pub struct DeathEvent {
    pub final_blow: AttackEvent,
    pub damage_taken: f32,
}