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
            PostUpdate,
            (kill_on_sunrise, damage::process_attacks, damage::do_death).chain(),
        );
    }
}

#[derive(Clone, Copy, Component)]
pub struct KillOnSunrise;

pub fn process_attacks(
    mut attack_reader: EventReader<AttackEvent>,
    mut death_writer: EventWriter<DeathEvent>,
    mut damaged_writer: EventWriter<DamageTakenEvent>,
    mut set: ParamSet<(
        Query<(&Combatant, &GlobalTransform)>,
        Query<(&mut Combatant, &mut Velocity, Option<&Mass>)>,
    )>,
    name_query: Query<&Name>,
    mut buffer: Local<Vec<(f32, Entity, AttackEvent)>>,
) {
    const BASE_KNOCKBACK: f32 = 0.01; //rescale knockback so that knockback mult = 1 is sensible
    for attack in attack_reader.read() {
        let mut query = set.p0();
        let Ok((target_info, gtf)) = query.get_inner(attack.target) else {
            warn!("attacking target without combatant and transform");
            continue;
        };
        let mut lens = query.transmute_lens::<&Combatant>();
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
        buffer.push((damage_taken, root, attack.clone()));
    }
    let mut update_query = set.p1();
    for (damage_taken, target, attack) in buffer.drain(..) {
        let Ok((mut target_combatant, mut v, opt_mass)) = update_query.get_mut(target) else {
            warn!("combatant is missing velocity");
            continue;
        };
        match target_combatant.as_mut() {
            Combatant::Root { health, .. } => {
                health.current = (health.current - damage_taken).max(0.0);
                opt_mass
                    .unwrap_or_default()
                    .add_impulse(attack.knockback, &mut v);
                if health.current == 0.0 {
                    //die
                    death_writer.send(DeathEvent {
                        final_blow: AttackEvent { target, ..attack },
                        damage_taken,
                    });
                }
            }
            Combatant::Child { .. } => {
                error!("root of combatant was a child");
            }
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
