use std::time::Instant;

use super::mesher::*;

use crate::world::chunk::*;
use crate::worldgen::GeneratedLODChunk;
use crate::world::{Level, *};
use bevy::{
    prelude::*,
    tasks::AsyncComputeTaskPool,
};



//there may be a cleaner way to do this, with some traits
//but I expect there will be significant differences between LOD and non-LOD meshing, so probably best to have separate functions entirely
pub fn queue_meshing_lod(
    query: Query<(Entity, &ChunkCoord, &LODLevel), (With<GeneratedLODChunk>, With<NeedsMesh>)>,
    level: Res<Level>,
    time: Res<Time>,
    mut timer: ResMut<MeshTimer>,
    mut commands: Commands,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    let mut len = 0;
    for (entity, coord, lod) in query.iter() {
        if let Some(chunks) = level.get_lod_chunks(lod.level) {
            if let Some(ctype) = chunks.get(coord) {
            if let LODChunkType::Full(chunk) = ctype.value() {
                //don't wait for neighbors since we won't have all neighbors generated, as there is a big hole in the middle of where we generate
                //TODO: greedy meshing is very important here
                let meshing = chunk.clone();
                len += 1;
                let task = pool.spawn(async move {
                    let mut data = MeshData {
                        verts: Vec::new(),
                        norms: Vec::new(),
                        tris: Vec::new(),
                        uvs: Vec::new(),
                        layer_idx: Vec::new(),
                        ao_level: Vec::new(),
                        scale: meshing.scale() as f32,
                        position: meshing.get_block_pos(ChunkIdx::new(0,0,0))
                    };
                    mesh_chunk_lod(&meshing, &mut data);
                    data
                });
                commands
                    .entity(entity)
                    .remove::<NeedsMesh>()
                    .insert(MeshTask { task });
            }
        }
    }
    }
    let duration = Instant::now().duration_since(now).as_millis();
    if len > 0 {
        println!(
            "queued mesh generation for {} chunks in {}ms",
            len, duration
        );
    }
}


fn mesh_chunk_lod(chunk: &LODChunk, data: &mut MeshData) {
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        let coord = ChunkIdx::from_usize(i);
        mesh_block_lod(&chunk, &chunk[i], coord, coord.to_vec3()*data.scale, data);
    }
}

fn mesh_block_lod(
    chunk: &LODChunk,
    b: &BlockType,
    coord: ChunkIdx,
    origin: Vec3,
    data: &mut MeshData,
) {
    if let BlockType::Empty = b {
        return;
    }
    if coord.z == CHUNK_SIZE_U8 - 1 {
        mesh_pos_z(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y, coord.z + 1)],
        BlockType::Empty
    ) {
        mesh_pos_z(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //negative z face
    if coord.z == 0 {
        mesh_neg_z(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y, coord.z - 1)],
        BlockType::Empty
    ) {
        mesh_neg_z(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //positive y face
    if coord.y == CHUNK_SIZE_U8 - 1 {
        mesh_pos_y(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y + 1, coord.z)],
        BlockType::Empty
    ) {
        mesh_pos_y(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //negative y face
    if coord.y == 0 {
        mesh_neg_y(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y - 1, coord.z)],
        BlockType::Empty
    ) {
        mesh_neg_y(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //positive x face
    if coord.x == CHUNK_SIZE_U8 - 1 {
        mesh_pos_x(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    } else if matches!(
        chunk[ChunkIdx::new(coord.x + 1, coord.y, coord.z)],
        BlockType::Empty
    ) {
        mesh_pos_x(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //negative x face
    if coord.x == 0 {
        mesh_neg_x(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    } else if matches!(
        chunk[ChunkIdx::new(coord.x - 1, coord.y, coord.z)],
        BlockType::Empty
    ) {
        mesh_neg_x(b, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
}