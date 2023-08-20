use bevy::prelude::*;

use crate::world::LevelSystemSet;

pub struct BevyUtilsPlugin;

impl Plugin for BevyUtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_timed_despawner.in_set(LevelSystemSet::Despawn),
        );
    }
}

#[derive(Component)]
pub struct TimedDespawner(pub Timer);

fn update_timed_despawner(
    mut commands: Commands,
    mut query: Query<(Entity, &mut TimedDespawner)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}
