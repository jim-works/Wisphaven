use bevy::prelude::*;

use util::direction::DirectionFlags;

use crate::{
    actors::block_actors::{FallingBlock, SpawnFallingBlockEvent},
    world::{
        events::{BlockUsedEvent, ChunkUpdatedEvent},
        BlockId, BlockType, Level, LevelSystemSet,
    },
};

pub struct FallPlugin;

impl Plugin for FallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, process_fall.in_set(LevelSystemSet::Main))
            .register_type::<FallOnUse>();
    }
}

#[derive(Component, Clone, Copy, Reflect, Default)]
#[reflect(Component, FromWorld)]
pub struct FallOnUse;

pub fn process_fall(
    mut fall_writer: EventWriter<SpawnFallingBlockEvent>,
    mut uses: EventReader<BlockUsedEvent>,
    fall_query: Query<&FallOnUse>,
    level: Res<Level>,
    id_query: Query<&BlockId>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    mut commands: Commands,
) {
    for used in uses.read() {
        if fall_query.get(used.block_used).is_ok() {
            level.set_block_entity(
                used.block_position,
                BlockType::Empty,
                &id_query,
                &mut update_writer,
                &mut commands,
            );
            fall_writer.send(SpawnFallingBlockEvent {
                position: used.block_position.to_vec3(),
                initial_velocity: Vec3::ZERO,
                falling_block: FallingBlock {
                    block: used.block_used,
                    place_on_landing: true,
                    impact_direcitons: DirectionFlags::all(),
                },
            });
        }
    }
}
