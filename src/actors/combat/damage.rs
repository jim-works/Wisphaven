use bevy::prelude::*;
use bevy_rapier3d::prelude::ExternalImpulse;

use super::*;

pub fn process_attacks (
    mut attack_reader: EventReader<AttackEvent>,
    mut death_writer: EventWriter<DeathEvent>,
    mut combat_query: Query<(&mut CombatInfo, Option<&mut ExternalImpulse>)>,
) {
    for attack in attack_reader.iter() {
        if let Ok((mut target_info, impulse)) = combat_query.get_mut(attack.target) {
            let damage_taken = calc_damage(attack, &target_info);
            if let Some(mut impulse) = impulse {
                impulse.impulse += attack.knockback*target_info.knockback_multiplier;
            }
            target_info.curr_health = (target_info.curr_health-damage_taken).max(0.0);
            info!("{:?} attacked {:?} for {} damage (inital damage {}). health: {}", attack.attacker, attack.target, damage_taken, attack.damage, target_info.curr_health);
            if target_info.curr_health == 0.0 {
                //die
                death_writer.send(DeathEvent { final_blow: *attack, damage_taken })
            }
        } else {
            warn!("tried to attack entity without combat info");
            continue;
        }
    }
}

pub fn calc_damage(attack: &AttackEvent, info: &CombatInfo) -> f32 {
    //curve sets damage multiplier between 0 and 2. infinite defense gives multiplier 0, -infinite defense gives multiplier 2
    //0 defense gives multiplier 1
    //TODO: maybe switch to sigmoid, I don't think I want armor to have this amount of diminishing returns.
    const DEFENSE_SCALE: f32 = 0.1;
    (1.0-(DEFENSE_SCALE*info.curr_defense)/(1.0+(DEFENSE_SCALE*info.curr_defense).abs()))*attack.damage
}

pub fn do_death(
    mut death_reader: EventReader<DeathEvent>,
    death_type: Query<&DeathInfo>,
    mut commands: Commands,
) {
    for event in death_reader.iter() {
        let dying_entity = event.final_blow.target; 
        if let Ok(death) = death_type.get(dying_entity) {
            match death.death_type {
                DeathType::Default => commands.entity(dying_entity).despawn_recursive(),
                DeathType::LocalPlayer => info!("Local player died!"),
                DeathType::RemotePlayer => info!("Remote player died!"),
                DeathType::Immortal => {}
            }
        }
    }
}