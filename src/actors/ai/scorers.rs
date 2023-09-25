use bevy::prelude::*;
use big_brain::prelude::*;

use crate::actors::AggroTargets;

pub struct ScorersPlugin;

impl Plugin for ScorersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            update_ranged_line_of_sight_scorer.in_set(BigBrainSet::Scorers),
        );
    }
}

//uses the current value of AggroTargets for the entity
//1.0 if in range and in line of sight, 0.0 otherwise
#[derive(Component, ScorerBuilder, Clone, Copy, Debug)]
pub struct AggroLineOfSightScorer {
    pub range: f32
}

fn update_ranged_line_of_sight_scorer(
    actor_query: Query<(&AggroTargets, &GlobalTransform)>,
    tf_query: Query<&GlobalTransform>,
    mut query: Query<(&Actor, &mut Score, &AggroLineOfSightScorer)>,
) {
    for (&Actor(actor), mut score, AggroLineOfSightScorer { range }) in query.iter_mut() {
        if let Ok((targets, actor_tf)) = actor_query.get(actor) {
            if let Some(target_tf) = targets.last().map(|t| tf_query.get(*t).ok()).flatten() {
                if target_tf.translation().distance_squared(actor_tf.translation()) <= range*range {
                    score.set(1.0);
                }
            }
        }
        score.set(0.0);
    }
}
