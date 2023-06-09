use bevy::prelude::*;

mod damage;
pub use damage::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AttackEvent>()
            .add_event::<DeathEvent>()
            .add_system(process_attacks.in_base_set(CoreSet::PostUpdate))
            // .add_system(test_attack.in_base_set(CoreSet::Update))
        ;
    }
}

#[derive(Component)]
pub struct CombatInfo {
    curr_health: f32,
    max_health: f32,
    curr_defense: f32,
    base_defense: f32,
}

#[derive(Component)]
pub struct DeathInfo {
    death_type: DeathType,
    //death_message: Option<&str>,
}

#[derive(Default)]
pub enum DeathType {
    #[default] Default,
    LocalPlayer,
    RemotePlayer,
}

impl CombatInfo {
    pub fn new(health: f32, defense: f32) -> CombatInfo {
        CombatInfo { curr_health: health, max_health: health, curr_defense: defense, base_defense: defense }
    }
}

#[derive(Clone, Copy)]
pub struct AttackEvent {
    attacker: Entity,
    target: Entity,
    damage: f32,
}

#[derive(Clone, Copy)]
pub struct DeathEvent {
    final_blow: AttackEvent,
    damage_taken: f32,
}