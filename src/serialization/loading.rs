use bevy::prelude::*;

use crate::{
    world::{
        chunk::{ChunkCoord, ChunkType},
        Level, BlockResources, BlockId, LevelData, events::ChunkUpdatedEvent,
    },
    worldgen::{ChunkNeedsGenerated, GeneratedChunk},
};

use super::{ChunkSaveFormat, NeedsLoading, SaveTimer, SavedToLoadedIdMap};
use super::db::*;

const LOADING_ENABLED: bool = true;

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
                LoadCommand {
                    position: *coord,
                    to_load: vec![ChunkTable::Terrain, ChunkTable::Buffers],
                }
            })
            .collect(),
    );
}

pub fn load_chunk_terrain(
    mut commands: Commands,
    mut events: EventReader<DataFromDBEvent>,
    mut tf_query: Query<&mut Transform>,
    level: Res<Level>,
    resources: Res<BlockResources>,
    map: Res<SavedToLoadedIdMap<BlockId>>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
) {
    let mut loaded = 0;
    for DataFromDBEvent(coord, data_vec) in events.iter().filter(|DataFromDBEvent(_, data)| {
        //even if there is no terrain/buffer, we will still have entries (just with an empty data vec)
        data.len() == 2 && data[0].0 == ChunkTable::Terrain && data[1].0 == ChunkTable::Buffers
    }) {
        let terrain_data = &data_vec[0].1;
        let buff_data = &data_vec[1].1;
        //do buffers before loading terrain, that way if there's both, we only generate the terrain mesh once.
        //first copy over the buffer so that it is applied when the chunk is added right after the terrain loads.
        if LOADING_ENABLED && !buff_data.is_empty() {
            match bincode::deserialize::<ChunkSaveFormat>(buff_data.as_slice()) {
                Ok(mut fmt) => {
                    fmt.map_to_loaded(&map);
                    level.add_rle_buffer(*coord, &fmt.into_buffer(&resources.registry, &mut commands), &mut commands, &mut update_writer)
                },
                Err(e) => error!("error deserializing chunk buffer at {:?}: {:?}", coord, e),
            }
        }
        //load terrain or mark as needing generation
        if let Some(entity) = level.get_chunk_entity(*coord) {
            if !LOADING_ENABLED || terrain_data.is_empty() {
                commands.entity(entity).insert(ChunkNeedsGenerated::Full);
            } else {
                match bincode::deserialize::<ChunkSaveFormat>(terrain_data.as_slice()) {
                    Ok(mut parsed) => {
                        parsed.map_to_loaded(&map);
                        let chunk = parsed.into_chunk(entity, &resources.registry, &mut commands);
                        let pos = chunk.position;
                        if let Ok(mut tf) = tf_query.get_mut(entity) {
                            tf.translation = chunk.position.to_vec3();
                        }
                        level.add_chunk(chunk.position, ChunkType::Full(chunk));
                        LevelData::update_chunk_only::<false>(entity, pos,&mut commands, &mut update_writer);
                        commands.entity(entity).insert(GeneratedChunk);
                        loaded += 1;
                    },
                    Err(e) => error!("error deserializing chunk terrain at {:?}: {:?}", coord, e),
                }
                
            }
        }
    }

    if loaded > 0 {
        info!("Loaded terrain for {} chunks.", loaded);
    }
}
