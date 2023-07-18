use bevy::prelude::*;

use crate::{
    actors::block_actors::{SpawnFallingBlockEvent, LandedFallingBlockEvent},
    world::{
        events::{BlockUsedEvent, ExplosionEvent},
        BlockId, BlockType, Level, LevelSystemSet,
    },
};

pub struct TNTPlugin;

impl Plugin for TNTPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (process_tnt,tnt_landed).in_set(LevelSystemSet::Main))
            .register_type::<TNTBlock>();
    }
}

#[derive(Component, Clone, Copy, Reflect, Default)]
#[reflect(Component)]
pub struct TNTBlock {
    pub explosion_strength: f32,
}

// pub fn process_tnt(
//     mut explosions: EventWriter<ExplosionEvent>,
//     mut uses: EventReader<BlockUsedEvent>,
//     tnt_query: Query<&TNTBlock>
// ) {
//     for used in uses.iter() {
//         if let Ok(tnt) = tnt_query.get(used.block_used) {
//             explosions.send(ExplosionEvent { radius: tnt.explosion_strength, origin: used.block_position });
//         }
//     }
// }

pub fn process_tnt(
    mut explosions: EventWriter<SpawnFallingBlockEvent>,
    mut uses: EventReader<BlockUsedEvent>,
    tnt_query: Query<&TNTBlock>,
    level: Res<Level>,
    id_query: Query<&BlockId>,
    mut commands: Commands,
) {
    for used in uses.iter() {
        if let Ok(_) = tnt_query.get(used.block_used) {
            level.set_block_entity(
                used.block_position,
                BlockType::Empty,
                &id_query,
                &mut commands,
            );
            explosions.send(SpawnFallingBlockEvent {
                position: used.block_position.center(),
                initial_velocity: Vec3::ZERO,
                block: used.block_used,
                place_on_landing: false,
            })
        }
    }
}

pub fn tnt_landed(
    mut explosions: EventWriter<ExplosionEvent>,
    tnt_query: Query<&TNTBlock>,
    mut reader: EventReader<LandedFallingBlockEvent>
) {
    for event in reader.iter() {
        if let Ok(tnt) = tnt_query.get(event.block) {
            explosions.send(ExplosionEvent { radius: tnt.explosion_strength, origin: event.position });
        }
    }
}