use ahash::HashSet;
use bevy::prelude::*;

use crate::{
    world::{chunk::{ChunkCoord, ChunkType}, Level},
    worldgen::GeneratedChunk,
};

use super::{NeedsSaving, SaveChunkEvent, SaveTimer, ChunkSaveFormat, LevelDB};

pub fn save_all (
    mut save_writer: EventWriter<SaveChunkEvent>,
    mut commands: Commands,
    mut timer: ResMut<SaveTimer>,
    time: Res<Time>,
    query: Query<(Entity, &ChunkCoord), (With<NeedsSaving>, With<GeneratedChunk>)>
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    for (entity, coord) in query.iter() {
        save_writer.send(SaveChunkEvent(*coord));
        commands.entity(entity).remove::<NeedsSaving>();
    }
}

pub fn do_saving(
    mut save_events: EventReader<SaveChunkEvent>,
    mut db: ResMut<LevelDB>,
    level: Res<Level>,
) {
    let mut saved = 0;
    //get unique coordinates
    let to_save = HashSet::from_iter(save_events.iter().map(
        |x| x.0
    ));
    let mut data = Vec::new();
        for coord in to_save {
            if let Some(chunk_ref) = level.get_chunk(coord) {
                if let ChunkType::Full(chunk) = chunk_ref.value() {
                    data.push((coord, ChunkSaveFormat::from(chunk).into_bits()));
                    saved += 1;
                }
            }
        }
    if let Some(err) = db.save_chunk_data(super::ChunkTable::Terrain, data) {
        error!("Error saving chunks: {:?}", err);
        return;
    }
    if saved > 0 {
        info!("Saved {} chunks.", saved);
    }
}