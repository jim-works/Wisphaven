use core::f32;

use bevy::prelude::*;

use crate::{
    physics::movement::{Mass, Velocity},
    world::atmosphere::DayStartedEvent,
};

use super::*;

pub struct DamagePlugin;

impl Plugin for DamagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            //todo - move to posttick
            PostUpdate,
            (
                kill_on_sunrise,
                process_attacks,
                (update_health, apply_knockback),
                do_death,
            )
                .chain(),
        );
    }
}

#[derive(Clone, Copy, Component)]
pub struct KillOnSunrise;

pub fn process_attacks(
    mut attack_reader: EventReader<AttackEvent>,
    mut damaged_writer: EventWriter<DamageTakenEvent>,
    mut target_query: Query<(&Combatant, &GlobalTransform)>,
    name_query: Query<&Name>,
) {
    const BASE_KNOCKBACK: f32 = 0.01; //rescale knockback so that knockback mult = 1 is sensible
    for attack in attack_reader.read() {
        let Ok((target_info, gtf)) = target_query.get_inner(attack.target) else {
            warn!("attacking target without combatant and transform");
            continue;
        };
        let mut lens = target_query.transmute_lens::<&Combatant>();
        let Some(damage_taken) = attack.damage.calc_recursive(&target_info, &lens.query()) else {
            error!("child combatant has no root!");
            continue;
        };

        let root = target_info
            .get_ancestor(&lens.query())
            .unwrap_or(attack.target);
        damaged_writer.send(DamageTakenEvent {
            attacker: attack.attacker,
            target: root,
            damage_taken: Damage {
                amount: damage_taken,
                ..attack.damage
            },
            knockback_impulse: attack.knockback,
            hit_location: gtf.translation(),
        });

        info!(
            "{:?} ({:?}) attacked {:?} ({:?}) for {} damage (inital damage {:?})",
            attack.attacker,
            name_query.get(attack.attacker).ok(),
            root,
            name_query.get(root).ok(),
            damage_taken,
            attack.damage,
        );
    }
}

fn update_health(
    mut reader: EventReader<AttackEvent>,
    mut writer: EventWriter<DeathEvent>,
    mut query: Query<(&mut Combatant, &mut Invulnerability)>,
    time: Res<Time<Fixed>>,
) {
    let current_time = time.elapsed();
    for attack in reader.read() {
        if let Ok((mut combatant, mut invulnerability)) = query.get_mut(attack.target) {
            if invulnerability.is_active(current_time) {
                continue;
            }
            invulnerability.on_hit(current_time);

            if let Combatant::Root { health, .. } = combatant.as_mut() {
                health.current = (health.current - attack.damage.amount).max(0.0);
                if health.current == 0.0 {
                    //die
                    writer.send(DeathEvent {
                        final_blow: AttackEvent {
                            target: attack.target,
                            ..*attack
                        },
                        damage_taken: attack.damage.amount,
                    });
                }
            }
        }
    }
}

fn apply_knockback(
    mut reader: EventReader<AttackEvent>,
    mut query: Query<(&mut Velocity, Option<&Mass>)>,
) {
    for AttackEvent {
        target, knockback, ..
    } in reader.read()
    {
        if let Ok((mut v, opt_mass)) = query.get_mut(*target) {
            opt_mass.unwrap_or_default().add_impulse(*knockback, &mut v);
        }
    }
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

fn kill_on_sunrise(
    query: Query<Entity, With<KillOnSunrise>>,
    mut writer: EventWriter<AttackEvent>,
    level_entity: Res<LevelEntity>,
    mut reader: EventReader<DayStartedEvent>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();
    for entity in query.iter() {
        writer.send(AttackEvent {
            attacker: level_entity.0,
            target: entity,
            damage: Damage {
                amount: f32::INFINITY,
                dtype: DamageType::HPRemoval,
            },
            knockback: Vec3::ZERO,
        });
    }
}
