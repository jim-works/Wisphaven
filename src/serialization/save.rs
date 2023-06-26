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
    query: Query<(Entity, &ChunkCoord), (With<NeedsSaving>, With<GeneratedChunk>)>,
    level: Res<Level>
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    for (entity, coord) in query.iter() {
        save_writer.send(SaveChunkEvent(*coord));
        commands.entity(entity).remove::<NeedsSaving>();
    }
    for buf_ref in level.buffer_iter() {
        save_writer.send(SaveChunkEvent(*buf_ref.key()));
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
                    data.push((super::ChunkTable::Terrain, coord, ChunkSaveFormat::from(chunk).into_bits()));
                    saved += 1;
                } else if let Some(buffer) = level.take_buffer(&coord) {
                    data.push((super::ChunkTable::Buffers, coord, ChunkSaveFormat::from((coord, buffer.1.as_ref())).into_bits()));
                }
            }
        }
    if saved > 0 {
        db.save_chunk_data(data);
        info!("Queued saving for {} chunks.", saved);
    }
}