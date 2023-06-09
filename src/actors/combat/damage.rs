use bevy::prelude::*;

use super::*;

pub fn process_attacks (
    mut attack_reader: EventReader<AttackEvent>,
    mut death_writer: EventWriter<DeathEvent>,
    mut combat_query: Query<&mut CombatInfo> 
) {
    for attack in attack_reader.iter() {
        if let Ok(mut target_info) = combat_query.get_mut(attack.target) {
            let damage_taken = calc_damage(attack, &target_info);
            target_info.curr_health = (target_info.curr_health-damage_taken).max(0.0);
            println!("{:?} attacked {:?} for {} damage (inital damage {}). health: {}", attack.attacker, attack.target, damage_taken, attack.damage, target_info.curr_health);
            if target_info.curr_health == 0.0 {
                //die
                death_writer.send(DeathEvent { final_blow: attack.clone(), damage_taken })
            }
        } else {
            println!("tried to attack entity without combat info");
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

// pub fn test_attack (
//     mut attack_writer: EventWriter<AttackEvent>,
//     combat_query: Query<Entity, With<CombatInfo>>
// ) {
//     for attacker in combat_query.iter() {
//         for target in combat_query.iter() {
//             if attacker != target {
//                 attack_writer.send(AttackEvent { attacker, target, damage: 1.0 })
//             }
//         }
//     }
// }