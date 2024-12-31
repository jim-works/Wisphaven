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
        app.add_event::<TriggerDamageEvent>();
    }
}

#[derive(Clone, Copy, Component)]
pub struct KillOnSunrise;

#[derive(Event)]
struct TriggerDamageEvent(DamageTakenEvent);

fn process_attacks(
    mut attack_reader: EventReader<AttackEvent>,
    mut damaged_writer: EventWriter<TriggerDamageEvent>,
    mut target_query: Query<(&Combatant, &GlobalTransform)>,
) {
    const BASE_KNOCKBACK: f32 = 0.01; //rescale knockback so that knockback mult = 1 is sensible
    for attack in attack_reader.read() {
        let Ok((target_info, gtf)) = target_query.get_inner(attack.target) else {
            warn!("attacking target without combatant and transform");
            continue;
        };
        let mut lens = target_query.transmute_lens::<&Combatant>();
        let Some(damage_taken) = attack.damage.calc_recursive(target_info, &lens.query()) else {
            error!("child combatant has no root!");
            continue;
        };

        let root = target_info
            .get_ancestor(&lens.query())
            .unwrap_or(attack.target);
        damaged_writer.send(TriggerDamageEvent(DamageTakenEvent {
            attacker: attack.attacker,
            target: root,
            damage: Damage {
                amount: damage_taken,
                ..attack.damage
            },
            knockback_impulse: attack.knockback,
            hit_location: gtf.translation(),
        }));
    }
}

fn update_health(
    mut reader: EventReader<TriggerDamageEvent>,
    mut damage_writer: EventWriter<DamageTakenEvent>,
    mut death_writer: EventWriter<DeathEvent>,
    mut query: Query<(&mut Combatant, &mut Invulnerability)>,
    name_query: Query<&Name>,
    time: Res<Time<Fixed>>,
) {
    let current_time = time.elapsed();
    for TriggerDamageEvent(attack) in reader.read() {
        if let Ok((mut combatant, mut invulnerability)) = query.get_mut(attack.target) {
            if invulnerability.is_active(current_time) {
                info!(
                    "{:?} tried to attack {:?} for {} damage, but they were invulnerable",
                    name_query.get(attack.attacker).map_err(|_| attack.attacker),
                    name_query.get(attack.target).map_err(|_| attack.target),
                    attack.damage.amount,
                );
                continue;
            }
            invulnerability.on_hit(current_time);

            // children are not sent to this function, so not doing a recursive check is ok
            if let Combatant::Root { health, .. } = combatant.as_mut() {
                health.current = (health.current - attack.damage.amount).max(0.0);
                info!(
                    "{:?} attacked {:?} for {} damage (new health={})",
                    name_query.get(attack.attacker).map_err(|_| attack.attacker),
                    name_query.get(attack.target).map_err(|_| attack.target),
                    attack.damage.amount,
                    health.current
                );
                damage_writer.send(*attack);
                if health.current <= 0.0 {
                    //die
                    death_writer.send(DeathEvent {
                        final_blow: *attack,
                        damage_taken: attack.damage.amount,
                    });
                }
            }
        } else {
            warn!("tried to update health on invalid entity");
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
    death_type: Query<(&DeathInfo, Option<&Name>)>,
    child_query: Query<(Entity, &Combatant, &DeathInfo)>,
    parent_query: Query<&Combatant>,
    mut commands: Commands,
) {
    for event in death_reader.read() {
        let dying_entity = event.final_blow.target;
        //todo - this is really inefficient. relations!!
        //  or just maintain a list of children for each combatant (bleh)
        for (child_entity, combatant, death) in child_query.iter() {
            if combatant.has_ancestor(dying_entity, &parent_query) {
                entity_die(child_entity, death, &mut commands);
            }
        }
        if let Ok((death, name)) = death_type.get(dying_entity) {
            info!("{:?} died", name);
            entity_die(dying_entity, death, &mut commands);
        }
    }
}

fn entity_die(entity: Entity, death: &DeathInfo, commands: &mut Commands) {
    match death.death_type {
        DeathType::Default => commands.entity(entity).despawn_recursive(),
        DeathType::LocalPlayer => {
            info!("Local Player died");
            commands.entity(entity).despawn_recursive();
        }
        DeathType::RemotePlayer => info!("Remote player died!"),
        DeathType::Immortal => {}
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
