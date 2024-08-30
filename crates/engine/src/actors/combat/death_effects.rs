use bevy::prelude::*;

use crate::actors::abilities::stamina::Stamina;

use super::DeathEvent;

pub struct DeathEffectsPlugin;

impl Plugin for DeathEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, restore_stamina_on_death);
    }
}

#[derive(Component, Clone, Copy)]
pub struct RestoreStaminaOnKill {
    //todo - stronger enemies should restore more stamina
    pub amount: f32,
}

fn restore_stamina_on_death(
    mut reader: EventReader<DeathEvent>,
    mut query: Query<(&mut Stamina, &RestoreStaminaOnKill)>,
) {
    for DeathEvent {
        final_blow,
        damage_taken: _,
    } in reader.read()
    {
        if let Ok((mut stamina, effect)) = query.get_mut(final_blow.attacker) {
            stamina.change(effect.amount);
        }
    }
}
