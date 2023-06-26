use bevy::prelude::*;

use crate::{
    world::{
        chunk::{ChunkCoord, ChunkType},
        Level,
    },
    worldgen::{ChunkNeedsGenerated, GeneratedChunk},
};

use super::{ChunkSaveFormat, ChunkTable, DataFromDBEvent, LevelDB, NeedsLoading, SaveTimer};

pub fn queue_terrain_loading(
    mut commands: Commands,
    mut db: ResMut<LevelDB>,
    query: Query<(Entity, &ChunkCoord), With<NeedsLoading>>,
    timer: Res<SaveTimer>,
) {
    //timer gets updating in saving system, so loading will happen after
    if !timer.0.finished() {
        return;
    }
    db.load_chunk_data(
        query
            .iter()
            .map(move |(entity, coord)| {
                commands.entity(entity).remove::<NeedsLoading>();
                (vec![ChunkTable::Terrain, ChunkTable::Buffers], *coord)
            })
            .collect(),
    );
}

pub fn load_chunk_terrain(
    mut commands: Commands,
    mut events: EventReader<DataFromDBEvent>,
    mut tf_query: Query<&mut Transform>,
    level: Res<Level>,
) {
    let mut loaded = 0;
    for DataFromDBEvent(coord, data_vec) in events.iter().filter(|DataFromDBEvent(_, v)| {
        //we only want events that have only the terrain and buffer
        v.len() == 2 && v[0].0 == ChunkTable::Terrain && v[1].0 == ChunkTable::Buffers
    }) {
        let terrain_data = &data_vec[0].1;
        let buff_data = &data_vec[1].1;
        //first copy over the buffer so that it is applied when the chunk is added right after the terrain loads.
        if !buff_data.is_empty() {
            match <&[u8] as TryInto<ChunkSaveFormat>>::try_into(buff_data.as_slice()) {
                Ok(fmt) => level.add_rle_buffer(*coord, &fmt.data, &mut commands),
                Err(e) => error!("error deserializing chunk buffer at {:?}: {:?}", coord, e),
            }
        }
        //load terrain or mark as needing generation
        if let Some(entity) = level.get_chunk_entity(*coord) {
            if terrain_data.is_empty() {
                commands.entity(entity).insert(ChunkNeedsGenerated::Full);
            } else {
                if let Ok(parsed) =
                    <&[u8] as TryInto<ChunkSaveFormat>>::try_into(terrain_data.as_slice())
                {
                    let chunk = parsed.into_chunk(entity);
                    if let Ok(mut tf) = tf_query.get_mut(entity) {
                        tf.translation = chunk.position.to_vec3();
                    }
                    level.add_chunk(chunk.position, ChunkType::Full(chunk));
                    Level::update_chunk_only::<false>(entity, &mut commands);
                    commands.entity(entity).insert(GeneratedChunk);
                    loaded += 1;
                }
            }
        }
    }

    if loaded > 0 {
        info!("Loaded terrain for {} chunks.", loaded);
    }
}
