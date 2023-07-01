use ahash::HashSet;
use bevy::prelude::*;

use crate::{
    world::{
        chunk::{ChunkCoord, ChunkType},
        Level,
    },
    worldgen::GeneratedChunk,
};

use super::{ChunkSaveFormat, LevelDB, NeedsSaving, SaveChunkEvent, SaveCommand, SaveTimer};

pub fn save_all(
    mut save_writer: EventWriter<SaveChunkEvent>,
    mut commands: Commands,
    mut timer: ResMut<SaveTimer>,
    time: Res<Time>,
    query: Query<(Entity, &ChunkCoord), (With<NeedsSaving>, With<GeneratedChunk>)>,
    level: Res<Level>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    for (entity, coord) in query.iter() {
        save_writer.send(SaveChunkEvent(*coord));
    }
    for buf_ref in level.buffer_iter() {
        save_writer.send(SaveChunkEvent(*buf_ref.key()));
    }
}

pub fn do_saving(
    mut save_events: EventReader<SaveChunkEvent>,
    mut db: ResMut<LevelDB>,
    level: Res<Level>,
    mut commands: Commands,
) {
    let mut saved = 0;
    //get unique coordinates
    let to_save = HashSet::from_iter(save_events.iter().map(|x| x.0));
    let mut save_data = Vec::new();
    for coord in to_save {
        if let Some(chunk_ref) = level.get_chunk(coord) {
            match chunk_ref.value() {
                ChunkType::Full(chunk) => {
                    if let Some(mut ec) = commands.get_entity(chunk.entity) {
                        save_data.push(SaveCommand(
                            super::ChunkTable::Terrain,
                            coord,
                            ChunkSaveFormat::from(chunk).into_bits(),
                        ));
                        saved += 1;
                        ec.remove::<NeedsSaving>();
                    }
                }
                ChunkType::Ungenerated(id) => {
                    if let Some(mut ec) = commands.get_entity(*id) {
                        ec.remove::<NeedsSaving>();
                    }
                }
            }
        }
        if let Some(buffer) = level.get_buffer(&coord) {
            save_data.push(SaveCommand(
                super::ChunkTable::Buffers,
                coord,
                ChunkSaveFormat::from((coord, buffer.value().as_ref())).into_bits(),
            ));
        }
    }
    if saved > 0 {
        db.save_chunk_data(save_data);
        debug!("Queued saving for {} chunks.", saved);
    }
}
