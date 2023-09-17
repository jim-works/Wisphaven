use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{
    actors::{skeleton_pirate::SpawnSkeletonPirateEvent, world_anchor::WorldAnchor},
    world::{Level, BlockCoord},
};

pub struct WavesPlugin;

impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                on_begin_assault.run_if(resource_added::<WorldAnchor>()),
                spawn_wave.run_if(in_state(WaveState::Spawning)),
            ),
        )
        .add_state::<WaveState>();
    }
}

#[derive(States, Default, Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub enum WaveState {
    #[default]
    NotStarted,
    Spawning,
    Finished,
}

#[derive(Resource)]
pub struct WaveInfo {
    pub current_wave: u32,
}

impl Default for WaveInfo {
    fn default() -> Self {
        Self {
            current_wave: Default::default(),
        }
    }
}

fn on_begin_assault(mut commands: Commands, mut next_state: ResMut<NextState<WaveState>>) {
    info!("Assault begins!");
    commands.insert_resource(WaveInfo::default());
    next_state.set(WaveState::Spawning);
}

fn spawn_wave(
    mut skeleton_pirate_spawner: EventWriter<SpawnSkeletonPirateEvent>,
    level: Res<Level>,
    mut next_state: ResMut<NextState<WaveState>>,
    anchor_query: Query<&GlobalTransform, With<WorldAnchor>>
) {
    if let Ok(tf) = anchor_query.get_single() {
        info!("trying to spawn...");
        //TODO: should check for a clear area instead of a single block (and be improved in general)
        //      ++ should check downwards so that they don't spawn in the air (maybe this kind of code could be handled by the actor? idk)
        //spawn in circle, check vertical until we find an empty block to spawn on
        const COUNT: i32 = 5;
        const RADIUS: f32 = 25.0;
        const MAX_CHECK: i32 = 100;
        const DELTA_ANGLE: f32 = 2.0*PI/COUNT as f32;
        let center = tf.translation();
        for i in 0..COUNT {
            let spawn_point = BlockCoord::from(center + RADIUS*Vec3::new((i as f32*DELTA_ANGLE).cos(), 0.0, (i as f32*DELTA_ANGLE).sin()));
            let mut check_point = spawn_point;
            while check_point.y - spawn_point.y < MAX_CHECK && level.get_block_entity(check_point).is_some() {
                check_point.y += 1;
            }
            if level.get_block_entity(check_point).is_none() {
                //we have a clear spot to spawn
                skeleton_pirate_spawner.send(SpawnSkeletonPirateEvent { location: GlobalTransform::from_translation(spawn_point.center()) });
                info!("Spawned pirate!");
            }
        }
        next_state.set(WaveState::Finished);
    }
}
