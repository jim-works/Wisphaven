use bevy::{prelude::*, utils::HashSet};

use crate::{world::{chunk::{ChunkCoord, ChunkType, LODChunk, LODChunkType}, Level}, worldgen::worldgen::ChunkNeedsGenerated};

#[derive(Component)]
pub struct ChunkLoader {
    pub radius: i32,
    pub lod_levels: i32,
}

pub struct DespawnChunkEvent(pub Entity);

pub fn do_loading(mut commands: Commands, mut level: ResMut<Level>, mut despawn_writer: EventWriter<DespawnChunkEvent>, 
    loader_query: Query<(&GlobalTransform, &ChunkLoader), Changed<GlobalTransform>>) {
    //load all in range
    let mut loaded_chunks = HashSet::new();
    let mut removed = Vec::new();
    for (transform, loader) in loader_query.iter() {
        let base_coord = ChunkCoord::from(transform.translation());
        for x in (base_coord.x - loader.radius)..(base_coord.x + loader.radius + 1) {
            for y in (base_coord.y - loader.radius)..(base_coord.y + loader.radius + 1) {
                for z in (base_coord.z - loader.radius)..(base_coord.z + loader.radius + 1) {
                    let test_coord = ChunkCoord::new(x,y,z);
                    loaded_chunks.insert(test_coord);
                    if !level.contains_chunk(test_coord) {
                        //chunk not loaded, load it!
                        let id = commands.spawn((test_coord, ChunkNeedsGenerated::Full)).id();
                        level.add_chunk(test_coord, ChunkType::Ungenerated(id), &mut removed);
                        
                    }
                }
            }
        }
        load_lod(1, &mut commands, &mut level, &transform, &loader, &mut removed);
        load_lod(2, &mut commands, &mut level, &transform, &loader, &mut removed);
    }
    despawn_writer.send_batch(removed.into_iter().map(|x| DespawnChunkEvent(x)));
    // //unload all not in range
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

fn load_lod(lod_level: usize, commands: &mut Commands, level: &mut ResMut<Level>, transform: &GlobalTransform, loader: &ChunkLoader, removed: &mut Vec<Entity>) {
        let base_coord = ChunkCoord::from(transform.translation()/LODChunk::level_to_scale(lod_level) as f32);
        let mut removed = Vec::new();
        for x in (base_coord.x - loader.radius)..(base_coord.x + loader.radius + 1) {
            for y in (base_coord.y - loader.radius)..(base_coord.y + loader.radius + 1) {
                for z in (base_coord.z - loader.radius)..(base_coord.z + loader.radius + 1) {
                    //don't generate in the center, where more detailed chunks will be
                    // let no_radius = loader.radius/2;
                    // if base_coord.x-no_radius <= x && x <= base_coord.x+no_radius
                    //     && base_coord.y-no_radius <= y && y <= base_coord.y+no_radius
                    //     && base_coord.z-no_radius <= z && z <= base_coord.z+no_radius {
                    //         continue;
                    //     }
                    let test_coord = ChunkCoord::new(x,y,z);
                    if !level.contains_lod_chunk(lod_level, test_coord) {
                        //chunk not loaded, load it!
                        let id = commands.spawn((test_coord, ChunkNeedsGenerated::LOD(lod_level))).id();
                        level.add_lod_chunk(test_coord, crate::world::chunk::LODChunkType::Ungenerated(id, lod_level), &mut removed);
                        
                    }
                }
            }
        }
}

pub fn unload_all(input: Res<Input<KeyCode>>, mut level: ResMut<Level>, mut despawn_writer: EventWriter<DespawnChunkEvent>) {
    if input.just_pressed(KeyCode::Apostrophe) {
    //unload all not in range
    let mut to_unload = Vec::new();
    let mut to_unload_lod = Vec::new();
    for c in level.chunks_iter() {
        let key = c.key().clone();
        to_unload.push(key);
    }
    for i in 0..level.get_lod_levels() {
    for c in level.get_lod_chunks(i).unwrap() {
        let key = c.key().clone();
        let level = c.value();
        match level {
            LODChunkType::Ungenerated(_, level) => to_unload_lod.push((*level,key)),
            LODChunkType::Full(f) => to_unload_lod.push((f.level,key)),
            _ => {}
        };
        
    }
    }
    for coord in to_unload {
        if let Some((_,ctype)) = level.remove_chunk(coord) {
            match ctype {
                ChunkType::Ungenerated(id) => despawn_writer.send(DespawnChunkEvent(id)),
                ChunkType::Full(c) => despawn_writer.send(DespawnChunkEvent(c.entity)),
            }
        }
    }
    for (lod_level, coord) in to_unload_lod {
        if let Some((_,lodtype)) = level.remove_lod_chunk(lod_level, coord) {
            match lodtype {
                LODChunkType::Ungenerated(id,_) => despawn_writer.send(DespawnChunkEvent(id)),
                LODChunkType::Full(c) => despawn_writer.send(DespawnChunkEvent(c.entity)),
                _ => {}
            }
        }
    }
    }
}

pub fn despawn_chunks(mut commands: Commands, mut despawn_reader: EventReader<DespawnChunkEvent>) {
    for e in despawn_reader.iter() {
        if let Some(mut ec) = commands.get_entity(e.0) {
            ec.despawn();
        }
    }
}

