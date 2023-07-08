use bevy::prelude::*;

use crate::world::{events::{BlockUsedEvent, ExplosionEvent}, LevelSystemSet};

pub struct TNTPlugin;

impl Plugin for TNTPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(process_tnt.in_set(LevelSystemSet::Main))
        ;
    }
}

#[derive(Component, Clone, Copy)]
pub struct TNTBlock {
    pub explosion_strength: f32
}

pub fn process_tnt(
    mut explosions: EventWriter<ExplosionEvent>,
    mut uses: EventReader<BlockUsedEvent>,
    tnt_query: Query<&TNTBlock>
) {
    for used in uses.iter() {
        if let Ok(tnt) = tnt_query.get(used.block_used) {
            explosions.send(ExplosionEvent { radius: tnt.explosion_strength, origin: used.block_position });
        }
    }
}