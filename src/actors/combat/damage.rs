use bevy::prelude::*;

use crate::physics::movement::{Mass, Velocity};

use super::*;

pub fn process_attacks(
    mut attack_reader: EventReader<AttackEvent>,
    mut death_writer: EventWriter<DeathEvent>,
    mut damaged_writer: EventWriter<DamageTakenEvent>,
    mut combat_query: Query<(&mut CombatInfo, Option<(&Mass, &mut Velocity)>)>,
    name_query: Query<&Name>,
) {
    const BASE_KNOCKBACK: f32 = 0.01; //rescale knockback so that knockback mult = 1 is sensible
    for attack in attack_reader.read() {
        if let Ok((mut target_info, impulse)) = combat_query.get_mut(attack.target) {
            let damage_taken = calc_damage(attack, &target_info);
            let knockback_impulse =
                attack.knockback * target_info.knockback_multiplier * BASE_KNOCKBACK;
            damaged_writer.send(DamageTakenEvent {
                attacker: attack.attacker,
                target: attack.target,
                damage_taken: Damage {
                    amount: damage_taken,
                },
                knockback_impulse: attack.knockback,
            });
            if let Some((mass, mut v)) = impulse {
                mass.add_impulse(knockback_impulse, &mut v);
            }
            target_info.curr_health = (target_info.curr_health - damage_taken).max(0.0);
            info!(
                "{:?} ({:?}) attacked {:?} ({:?}) for {} damage (inital damage {:?}). health: {}",
                attack.attacker,
                name_query.get(attack.attacker).ok(),
                attack.target,
                name_query.get(attack.target).ok(),
                damage_taken,
                attack.damage,
                target_info.curr_health
            );
            if target_info.curr_health == 0.0 {
                //die
                death_writer.send(DeathEvent {
                    final_blow: *attack,
                    damage_taken,
                })
            }
        }
    }
}

pub fn calc_damage(attack: &AttackEvent, info: &CombatInfo) -> f32 {
    //curve sets damage multiplier between 0 and 2. infinite defense gives multiplier 0, -infinite defense gives multiplier 2
    //0 defense gives multiplier 1
    //TODO: maybe switch to sigmoid, I don't think I want armor to have this amount of diminishing returns.
    const DEFENSE_SCALE: f32 = 0.1;
    (1.0 - (DEFENSE_SCALE * info.curr_defense) / (1.0 + (DEFENSE_SCALE * info.curr_defense).abs()))
        * attack.damage.amount
}

pub fn do_death(
    mut death_reader: EventReader<DeathEvent>,
    death_type: Query<&DeathInfo>,
    mut commands: Commands,
) {
    for event in death_reader.read() {
        let dying_entity = event.final_blow.target;
        if let Ok(death) = death_type.get(dying_entity) {
            match death.death_type {
                DeathType::Default => commands.entity(dying_entity).despawn_recursive(),
                DeathType::LocalPlayer => {
                    info!("Local Player died");
                    commands.entity(dying_entity).despawn_recursive();
                }
                DeathType::RemotePlayer => info!("Remote player died!"),
                DeathType::Immortal => {}
            }
        }
    }
}
