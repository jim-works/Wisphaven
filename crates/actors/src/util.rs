use bevy::prelude::*;
use engine::actors::AggroTargets;
use util::plugin::SmoothLookTo;

pub(crate) struct ActorUtilPlugin;

impl Plugin for ActorUtilPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_smooth_look_to_aggro_target);
    }
}

#[derive(Component)]
//requires SmoothLookTo
pub(crate) struct SmoothLookToAggroTarget {
    pub source: Entity,
}

fn update_smooth_look_to_aggro_target(
    mut looker_query: Query<(
        Entity,
        &SmoothLookToAggroTarget,
        &mut SmoothLookTo,
        &GlobalTransform,
    )>,
    source_query: Query<&AggroTargets>,
    target_query: Query<&GlobalTransform>,
    mut commands: Commands,
) {
    for (looker_entity, source, mut looker, looker_gtf) in looker_query.iter_mut() {
        let Ok(source_entity) = source_query.get(source.source) else {
            warn!("SmoothLookToAggroTarget source does not have AggroTargets! removing...");
            commands.get_entity(looker_entity).map(|mut ec| {
                ec.remove::<SmoothLookToAggroTarget>();
            });
            continue;
        };
        if let Some(target_gtf) = source_entity
            .current_target()
            .and_then(|target_entity| target_query.get(target_entity).ok())
        {
            looker.enabled = true;
            looker.forward = target_gtf.translation() - looker_gtf.translation();
        } else {
            looker.enabled = false;
        }
    }
}
