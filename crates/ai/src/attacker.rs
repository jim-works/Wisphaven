use bevy::{math::FloatPow, prelude::*};
use big_brain::prelude::*;
use engine::{
    actors::{AggroTargets, team::Team},
    items::inventory::{Inventory, ItemAction, ItemTargetPosition},
};
use interfaces::scheduling::LevelSystemSet;

// can probably refactor engine/actors/ai here, be mindful of this crate's dependencies though

pub(crate) struct AttackerAIPlugin;

impl Plugin for AttackerAIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            update_closest_enemy_aggro.in_set(LevelSystemSet::Tick),
        )
        .add_systems(
            FixedUpdate,
            update_use_item_action
                .in_set(BigBrainSet::Actions)
                .in_set(LevelSystemSet::Tick),
        );
    }
}

#[derive(Component, Clone, Copy, Default)]
#[require(AggroTargets, Team, GlobalTransform)]
pub struct AggroClosestEnemy {
    pub range: f32,
    pub priority: i32,
    pub target: Option<Entity>,
}

#[derive(Component, Debug, ActionBuilder, Copy, Clone)]
pub struct UseItemAction {
    pub slot: usize,
}

fn update_closest_enemy_aggro(
    gtf_query: Query<&GlobalTransform>,
    combatant_query: Query<(Entity, &GlobalTransform, &Team)>,
    mut aggro_query: Query<(
        &mut AggroTargets,
        &GlobalTransform,
        &Team,
        &mut AggroClosestEnemy,
    )>,
) {
    for (mut aggro_targets, aggro_gtf, aggro_team, mut aggro_behavior) in aggro_query.iter_mut() {
        // if current target is in range, skip
        if let Some(target) = aggro_targets.current_target()
            && let Ok(target_gtf) = gtf_query.get(target)
            && target_gtf
                .translation()
                .distance_squared(aggro_gtf.translation())
                <= aggro_behavior.range.squared()
        {
            continue;
        }

        if let Some(cached_target) = aggro_behavior.target {
            //if aggro target is out of range, remove from AggroTargets and clear
            let mut cleared = false;
            if let Ok(target_gtf) = gtf_query.get(cached_target)
                && target_gtf
                    .translation()
                    .distance_squared(aggro_gtf.translation())
                    <= aggro_behavior.range.squared()
            {
                aggro_targets.pqueue.retain(|(t, _)| *t != cached_target);
                aggro_behavior.target = None;
                cleared = true;
            }

            // if aggro target was removed from AggroTargets by someone else, clear
            if !cleared
                && !aggro_targets
                    .pqueue
                    .iter()
                    .any(|(t, _)| *t == cached_target)
            {
                aggro_behavior.target = None;
            }
        }

        // skip if current target is still valid
        if aggro_behavior.target.is_some() {
            continue;
        }

        // add closest enemy in range to targets
        let (sqr_distance, closest_enemy) = combatant_query
            .iter()
            .filter(|(_, _, target)| aggro_team.can_hit(**target))
            .fold(
                (f32::MAX, None),
                |(curr_d, curr_enemy), (enemy_entity, enemy_gtf, _)| {
                    let d = aggro_gtf
                        .translation()
                        .distance_squared(enemy_gtf.translation());
                    if d < curr_d {
                        (d, Some(enemy_entity))
                    } else {
                        (curr_d, curr_enemy)
                    }
                },
            );

        if let Some(enemy) = closest_enemy
            && sqr_distance < aggro_behavior.range.squared()
        {
            aggro_targets.add_target(enemy, aggro_behavior.priority);
            aggro_behavior.target = Some(enemy);
        }
    }
}

// todo - aim for weapons with gravity??
fn update_use_item_action(
    mut action_query: Query<(&Actor, &mut ActionState, &UseItemAction)>,
    mut attacker_query: Query<(&mut Transform, &AggroTargets, &Inventory, &mut ItemAction)>,
    target_query: Query<&GlobalTransform>,
) {
    for (&Actor(actor), mut state, action) in action_query.iter_mut() {
        match *state {
            ActionState::Requested => {
                if let Ok((mut tf, aggro_targets, inv, mut item_action)) =
                    attacker_query.get_mut(actor)
                    && let Some(target_entity) = aggro_targets.current_target()
                    && let Ok(target_gtf) = target_query.get(target_entity)
                {
                    tf.look_at(target_gtf.translation(), Vec3::Y);
                    item_action.try_use(inv, ItemTargetPosition::Entity(actor));
                    *state = ActionState::Executing;
                } else {
                    info!("Cancelling use item action due to missing target or missing components");
                    *state = ActionState::Cancelled;
                }
            }
            ActionState::Executing => {
                //wait for animation to finish
                if let Ok((_, _, _, item_action)) = attacker_query.get_mut(actor) {
                    if matches!(*item_action, engine::items::inventory::ItemAction::None) {
                        *state = ActionState::Success;
                    }
                } else {
                    info!("Cancelling due to missing actor");
                    *state = ActionState::Cancelled;
                }
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
