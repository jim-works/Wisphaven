use bevy::prelude::*;
use interfaces::scheduling::LevelSystemSet;

use crate::actors::abilities::stamina::Stamina;

use super::DeathTrigger;

pub struct DeathEffectsPlugin;

impl Plugin for DeathEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(restore_stamina_on_death);
    }
}

#[derive(Component, Clone, Copy)]
pub struct RestoreStaminaOnKill {
    //todo - stronger enemies should restore more stamina
    pub amount: f32,
}

fn restore_stamina_on_death(
    trigger: Trigger<DeathTrigger>,
    mut query: Query<(&mut Stamina, &RestoreStaminaOnKill)>,
) {
    let final_blow = trigger.final_blow;
    info!("trying to restore stamina on kill {:?}", final_blow);
    if let Some(attacker_entity) = final_blow.attacker
        && let Ok((mut stamina, effect)) = query.get_mut(attacker_entity)
    {
        stamina.change(effect.amount);
        info!(
            "restoring {:?} stamina (total {:?})",
            effect.amount, stamina
        );
    }
}
