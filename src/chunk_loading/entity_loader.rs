use bevy::{prelude::*, utils::HashSet};

use crate::{world::{chunk::{ChunkCoord, ChunkType}, Level}, worldgen::worldgen::ChunkNeedsGenerated};

#[derive(Component)]
pub struct ChunkLoader {
    pub radius: i32,
}

pub struct DespawnChunkEvent(Entity);

pub fn do_loading(mut commands: Commands, mut level: ResMut<Level>, mut despawn_writer: EventWriter<DespawnChunkEvent>, loader_query: Query<(&GlobalTransform, &ChunkLoader)>) {
    //load all in range
    let mut loaded_chunks = HashSet::new();
    for (transform, loader) in loader_query.iter() {
        let base_coord = ChunkCoord::from(transform.translation());
        for x in (base_coord.x - loader.radius)..(base_coord.x + loader.radius + 1) {
            for y in (base_coord.y - loader.radius)..(base_coord.y + loader.radius + 1) {
                for z in (base_coord.z - loader.radius)..(base_coord.z + loader.radius + 1) {
                    let test_coord = ChunkCoord::new(x,y,z);
                    loaded_chunks.insert(test_coord);
                    if !level.chunks.contains_key(&test_coord) {
                        //chunk not loaded, load it!
                        let id = commands.spawn((test_coord, ChunkNeedsGenerated {})).id();
                        //level.add_chunk(test_coord, ChunkType::Ungenerated(id, 1));
                        
                    }
                }
            }
        }
    }
    //unload all not in range
    // let mut to_unload = Vec::new();
    // for c in level.chunks.iter() {
    //     let key = c.key().clone();
    //     if !loaded_chunks.contains(&key) {
    //         to_unload.push(key);
    //     }
    // }
    // for coord in to_unload {
    //     if let Some((_,ctype)) = level.chunks.remove(&coord) {
    //         match ctype {
    //             ChunkType::Ungenerated(id) => despawn_writer.send(DespawnChunkEvent(id)),
    //             ChunkType::Full(c) => despawn_writer.send(DespawnChunkEvent(c.entity)),
    //         }
    //     }
    // }
}

pub fn despawn_chunks(mut commands: Commands, mut despawn_reader: EventReader<DespawnChunkEvent>) {
    for e in despawn_reader.iter() {
        if let Some(mut ec) = commands.get_entity(e.0) {
            ec.despawn();
        }
    }
}

