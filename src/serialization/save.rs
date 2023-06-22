use bevy::prelude::*;

use crate::{
    world::{chunk::{ChunkCoord, ChunkType}, Level},
    worldgen::GeneratedChunk,
};

use super::{ChunkDB, HeedEnv, NeedsSaving};

pub fn do_saving(
    mut commands: Commands,
    save_query: Query<(Entity, &ChunkCoord), (With<NeedsSaving>, With<GeneratedChunk>)>,
    heed_env: Res<HeedEnv>,
    chunk_db: Res<ChunkDB>,
    level: Res<Level>,
) {
    let mut saved = 0;
    if let Ok(mut wtxn) = heed_env.0.write_txn() {
        for (entity, coord) in save_query.iter() {
            if let Some(chunk_ref) = level.get_chunk(*coord) {
                if let ChunkType::Full(_chunk) = chunk_ref.value() {
                    if let Err(e) = chunk_db.0.put(&mut wtxn, coord, &[0]) {
                        error!("Error saving chunk {:?} {}", coord, e);
                        break;
                    } else {
                        saved += 1;
                    }
                    commands.entity(entity).remove::<NeedsSaving>();
                }
            }
        }
        if let Err(e) = wtxn.commit() {
            error!("Error saving chunks {}", e);
        }
    }
    if saved > 0 {
        info!("Saved {} chunks.", saved);
    }
}
