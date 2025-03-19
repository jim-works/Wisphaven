use bevy::{prelude::*, utils::HashMap};

use crate::{
    chunk::{ChunkCoord, ChunkType, LODChunk, LODChunkType},
    level::Level,
};

use interfaces::scheduling::*;

#[derive(Component, Clone, Debug)]
pub struct ChunkLoader {
    pub radius: ChunkCoord,
    pub lod_levels: i32,
    pub mesh: bool, //controls whether the chunk visuals are generated in the area around this loader
}

impl ChunkLoader {
    pub fn for_each_chunk(&self, mut f: impl FnMut(ChunkCoord)) {
        for x in -self.radius.x..self.radius.x + 1 {
            for y in -self.radius.y..self.radius.y + 1 {
                for z in -self.radius.z..self.radius.z + 1 {
                    (f)(ChunkCoord::new(x, y, z));
                }
            }
        }
    }
    //doesn't include chunks on the edge of the loader
    pub fn for_each_center_chunk(&self, mut f: impl FnMut(ChunkCoord)) {
        for x in -self.radius.x + 1..self.radius.x {
            for y in -self.radius.y + 1..self.radius.y {
                for z in -self.radius.z + 1..self.radius.z {
                    (f)(ChunkCoord::new(x, y, z));
                }
            }
        }
    }
    pub fn chunk_in_range(&self, origin: ChunkCoord, testing: ChunkCoord) -> bool {
        let diff = testing - origin;
        (diff.x <= self.radius.x && diff.x >= -self.radius.x)
            && (diff.y <= self.radius.y && diff.y >= -self.radius.y)
            && (diff.z <= self.radius.z && diff.z >= -self.radius.z)
    }
}

#[derive(Resource)]
pub struct ChunkLoadingTimer {
    pub timer: Timer,
}

#[derive(Event)]
pub struct DespawnChunkEvent {
    pub entity: Entity,
    pub coords: ChunkCoord,
}

pub fn do_loading(
    mut commands: Commands,
    level: Res<Level>,
    mut despawn_writer: EventWriter<DespawnChunkEvent>,
    loader_query: Query<(&GlobalTransform, &ChunkLoader)>,
    mut timer: ResMut<ChunkLoadingTimer>,
    time: Res<Time>,
    save_query: Query<&crate::chunk::NeedsSaving>,
    network_type: Res<State<NetworkType>>,
) {
    let _my_span = info_span!("do_loading", name = "do_loading").entered();
    timer.timer.tick(time.delta());
    if !timer.timer.finished() {
        return;
    }
    //load all in range
    let mut loaded_chunks = HashMap::new();
    let mut loaded_lods = Vec::new();
    for (transform, loader) in loader_query.iter() {
        let base_coord = ChunkCoord::from(transform.translation());
        loader.for_each_chunk(|coord| {
            let test_coord = coord + base_coord;
            //if any loader has it meshed, it needs to be meshed
            loaded_chunks
                .entry(test_coord)
                .and_modify(move |b| *b = *b || loader.mesh)
                .or_insert(loader.mesh);
        });
        for i in 1..loader.lod_levels as usize {
            let mut loaded_lod = HashMap::new();
            load_lod(i, &mut commands, &level, transform, loader, &mut loaded_lod);
            loaded_lods.push(loaded_lod);
        }
    }
    match network_type.get() {
        //chunks get pushed from the server to the client, so the client doesn't need to worry about loading
        NetworkType::Client => {}
        _ => {
            for (coord, mesh) in loaded_chunks.iter() {
                level.load_chunk(*coord, *mesh, &mut commands);
            }
        }
    }
    //unload all not in range
    let mut to_unload = Vec::new();
    for c in level.chunks_iter() {
        let key = *c.key();
        if !loaded_chunks.contains_key(&key) {
            match c.value() {
                ChunkType::Ungenerated(id) => {
                    if !save_query.contains(*id) {
                        to_unload.push((key, *id));
                    }
                }
                ChunkType::Full(c) => {
                    if !save_query.contains(c.entity) {
                        to_unload.push((key, c.entity));
                    }
                }
                ChunkType::Generating(_, c) => {
                    if !save_query.contains(c.entity) {
                        to_unload.push((key, c.entity));
                    }
                }
            }
        }
    }
    for (coords, entity) in to_unload {
        if level.remove_chunk(coords).is_some() {
            despawn_writer.send(DespawnChunkEvent { entity, coords });
        }
    }
    //unload lods (i=lod-1)
    let mut to_unload_lod = Vec::new();
    for (i, lods) in loaded_lods.iter().enumerate() {
        let lod_level = i + 1;
        if let Some(chunks) = level.get_lod_chunks(lod_level) {
            for c in chunks.iter() {
                let key = *c.key();
                if !lods.contains_key(&key) {
                    to_unload_lod.push((lod_level, key));
                }
            }
        }
    }
    for (lod, coords) in to_unload_lod {
        if let Some((_, lodtype)) = level.remove_lod_chunk(lod, coords) {
            match lodtype {
                LODChunkType::Ungenerated(id, _) => {
                    despawn_writer.send(DespawnChunkEvent { entity: id, coords })
                }
                LODChunkType::Full(c) => despawn_writer.send(DespawnChunkEvent {
                    entity: c.entity,
                    coords,
                }),
            };
        }
    }
}

fn load_lod(
    lod_level: usize,
    commands: &mut Commands,
    level: &Level,
    transform: &GlobalTransform,
    loader: &ChunkLoader,
    loaded_list: &mut HashMap<ChunkCoord, bool>,
) {
    let _my_span = info_span!("load_lod", name = "load_lod").entered();
    let base_coord = ChunkCoord::from(
        transform.translation() / LODChunk::level_to_scale(lod_level as u8) as f32,
    );
    for x in (base_coord.x - loader.radius.x)..(base_coord.x + loader.radius.x + 1) {
        for y in (base_coord.y - loader.radius.y)..(base_coord.y + loader.radius.y + 1) {
            for z in (base_coord.z - loader.radius.z)..(base_coord.z + loader.radius.z + 1) {
                //don't generate in the center, where more detailed chunks will be
                let no_radius = loader.radius / 2;
                if base_coord.x - no_radius.x <= x
                    && x <= base_coord.x + no_radius.x
                    && base_coord.y - no_radius.y <= y
                    && y <= base_coord.y + no_radius.y
                    && base_coord.z - no_radius.z <= z
                    && z <= base_coord.z + no_radius.z
                {
                    continue;
                }
                let test_coord = ChunkCoord::new(x, y, z);
                loaded_list
                    .entry(test_coord)
                    .and_modify(move |b| *b = *b || loader.mesh)
                    .or_insert(loader.mesh);
                if !level.contains_lod_chunk(lod_level, test_coord) {
                    //chunk not loaded, load it!
                    level.create_lod_chunk(test_coord, lod_level as u8, commands);
                }
            }
        }
    }
}

pub fn despawn_chunks(mut commands: Commands, mut despawn_reader: EventReader<DespawnChunkEvent>) {
    let _my_span = info_span!("despawn_chunks", name = "despawn_chunks").entered();
    for DespawnChunkEvent { entity, coords } in despawn_reader.read() {
        if let Some(ec) = commands.get_entity(*entity) {
            info!("despawning chunk at {:?}", coords);
            ec.despawn_recursive();
        }
    }
}
