use futures_lite::future;
use std::time::Instant;

use super::mesher::*;

use crate::world::chunk::*;
use crate::worldgen::worldgen::{GeneratedChunk, GeneratedLODChunk};
use crate::{
    util::Direction,
    world::{Level, *},
};
use bevy::{
    prelude::*,
    render::{mesh, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task},
};

use super::{SPAWN_MESH_TIME_BUDGET_COUNT};

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
                let mut neighbor_count = 0;
                let mut neighbors = [None, None, None, None, None, None];
                //i wish i could extrac this if let Some() shit into a function
                //but that makes the borrow checker angry
                for dir in Direction::iter() {
                    if let Some(ctype) = chunks.get(&coord.offset(dir)) {
                        if let LODChunkType::Full(neighbor) = ctype.value() {
                            neighbors[dir.to_idx()] = Some(neighbor.clone());
                            neighbor_count += 1;
                        }
                    }
                }
                if neighbor_count != 6 {
                    //don't mesh if all neighbors aren't ready yet
                    continue;
                }
                let meshing = chunk.clone();
                len += 1;
                let task = pool.spawn(async move {
                    let mut data = MeshData {
                        verts: Vec::new(),
                        norms: Vec::new(),
                        tris: Vec::new(),
                        scale: meshing.scale() as f32,
                        position: meshing.get_block_pos(ChunkIdx::new(0,0,0))
                    };
                    mesh_chunk_lod(&meshing, &neighbors, &mut data);
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


fn mesh_chunk_lod(chunk: &LODChunk, neighbors: &[Option<LODChunk>; 6], data: &mut MeshData) {
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        let coord = ChunkIdx::from_usize(i);
        mesh_block_lod(&chunk, neighbors, &chunk[i], coord, coord.to_vec3()*data.scale, data);
    }
}

fn mesh_block_lod(
    chunk: &LODChunk,
    neighbors: &[Option<LODChunk>; 6],
    b: &BlockType,
    coord: ChunkIdx,
    origin: Vec3,
    data: &mut MeshData,
) {
    if let BlockType::Empty = b {
        return;
    }
    if coord.z == CHUNK_SIZE_U8 - 1 {
        if match &neighbors[Direction::PosZ.to_idx()] {
            Some(c) => matches!(c[ChunkIdx::new(coord.x, coord.y, 0)], BlockType::Empty),
            _ => true,
        } {
            mesh_pos_z(origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y, coord.z + 1)],
        BlockType::Empty
    ) {
        mesh_pos_z(origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //negative z face
    if coord.z == 0 {
        if match &neighbors[Direction::NegZ.to_idx()] {
            Some(c) => matches!(
                c[ChunkIdx::new(coord.x, coord.y, CHUNK_SIZE_U8 - 1)],
                BlockType::Empty
            ),
            _ => true,
        } {
            mesh_neg_z(origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y, coord.z - 1)],
        BlockType::Empty
    ) {
        mesh_neg_z(origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //positive y face
    if coord.y == CHUNK_SIZE_U8 - 1 {
        if match &neighbors[Direction::PosY.to_idx()] {
            Some(c) => matches!(c[ChunkIdx::new(coord.x, 0, coord.z)], BlockType::Empty),
            _ => true,
        } {
            mesh_pos_y(origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y + 1, coord.z)],
        BlockType::Empty
    ) {
        mesh_pos_y(origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //negative y face
    if coord.y == 0 {
        if match &neighbors[Direction::NegY.to_idx()] {
            Some(c) => matches!(
                c[ChunkIdx::new(coord.x, CHUNK_SIZE_U8 - 1, coord.z)],
                BlockType::Empty
            ),
            _ => true,
        } {
            mesh_neg_y(origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y - 1, coord.z)],
        BlockType::Empty
    ) {
        mesh_neg_y(origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //positive x face
    if coord.x == CHUNK_SIZE_U8 - 1 {
        if match &neighbors[Direction::PosX.to_idx()] {
            Some(c) => matches!(c[ChunkIdx::new(0, coord.y, coord.z)], BlockType::Empty),
            _ => true,
        } {
            mesh_pos_x(origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x + 1, coord.y, coord.z)],
        BlockType::Empty
    ) {
        mesh_pos_x(origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //negative x face
    if coord.x == 0 {
        if match &neighbors[Direction::NegX.to_idx()] {
            Some(c) => matches!(
                c[ChunkIdx::new(CHUNK_SIZE_U8 - 1, coord.y, coord.z)],
                BlockType::Empty
            ),
            _ => true,
        } {
            mesh_neg_x(origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x - 1, coord.y, coord.z)],
        BlockType::Empty
    ) {
        mesh_neg_x(origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
}