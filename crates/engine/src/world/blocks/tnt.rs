use bevy::prelude::*;

use util::direction::DirectionFlags;

use crate::{
    actors::block_actors::{FallingBlock, LandedFallingBlockEvent, SpawnFallingBlockEvent},
    world::{
        events::{BlockUsedEvent, ChunkUpdatedEvent, ExplosionEvent},
        BlockId, BlockType, Level, LevelSystemSet,
    },
};

pub struct TNTPlugin;

impl Plugin for TNTPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (process_tnt, tnt_landed).in_set(LevelSystemSet::Main),
        )
        .register_type::<TNTBlock>();
    }
}

#[derive(Component, Clone, Copy, Reflect, Default)]
#[reflect(Component, FromWorld)]
pub struct TNTBlock {
    pub explosion_strength: f32,
}

pub fn process_tnt(
    mut explosions: EventWriter<SpawnFallingBlockEvent>,
    mut uses: EventReader<BlockUsedEvent>,
    tnt_query: Query<&TNTBlock>,
    level: Res<Level>,
    id_query: Query<&BlockId>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    mut commands: Commands,
) {
    for used in uses.read() {
        if tnt_query.get(used.block_used).is_ok() {
            level.set_block_entity(
                used.block_position,
                BlockType::Empty,
                &id_query,
                &mut update_writer,
                &mut commands,
            );
            explosions.send(SpawnFallingBlockEvent {
                position: used.block_position.center(),
                initial_velocity: Vec3::ZERO,
                falling_block: FallingBlock {
                    block: used.block_used,
                    place_on_landing: false,
                    impact_direcitons: DirectionFlags::all(),
                },
            });
        }
    }
}

pub fn tnt_landed(
    mut explosions: EventWriter<ExplosionEvent>,
    tnt_query: Query<&TNTBlock>,
    mut reader: EventReader<LandedFallingBlockEvent>,
) {
    for event in reader.read() {
        if let Ok(tnt) = tnt_query.get(event.falling_block.block) {
            explosions.send(ExplosionEvent {
                radius: tnt.explosion_strength,
                origin: event.position,
            });
        }
    }
}
