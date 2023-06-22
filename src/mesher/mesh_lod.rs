use std::time::Instant;

use super::generator::*;

use crate::util::Direction;
use crate::world::chunk::*;
use crate::world::{Level, *};
use crate::worldgen::GeneratedLODChunk;
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

//there may be a cleaner way to do this, with some traits
//but I expect there will be significant differences between LOD and non-LOD meshing, so probably best to have separate functions entirely
pub fn queue_meshing_lod(
    query: Query<(Entity, &ChunkCoord, &LODLevel), (With<GeneratedLODChunk>, With<NeedsMesh>)>,
    level: Res<Level>,
    time: Res<Time>,
    mut timer: ResMut<MeshTimer>,
    mut commands: Commands,
) {
    let _my_span = info_span!("queue_meshing_lod", name = "queue_meshing_lod").entered();
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
                        let mut data = ChunkMesh::new(meshing.scale() as f32);
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
            "queued mesh generation for {} lod chunks in {}ms",
            len, duration
        );
    }
}

fn mesh_chunk_lod(chunk: &LODChunk, data: &mut ChunkMesh) {
    let _my_span = info_span!("mesh_chunk_lod", name = "mesh_chunk_lod").entered();
    let registry = crate::world::get_block_registry();
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        let coord = ChunkIdx::from_usize(i);
        let block = chunk[i];
        match block {
            BlockType::Empty => {}
            BlockType::Basic(id) => mesh_block_lod(
                chunk,
                registry.get_block_mesh(id),
                coord,
                coord.to_vec3() * data.scale,
                data,
                registry,
            ),
            BlockType::Entity(_) => todo!(),
        }
    }
}

fn mesh_block_lod(
    chunk: &LODChunk,
    b: &BlockMesh,
    coord: ChunkIdx,
    origin: Vec3,
    data: &mut ChunkMesh,
    registry: &BlockRegistry,
) {
    if coord.z == CHUNK_SIZE_U8 - 1
        || should_mesh_face(
            registry,
            b,
            Direction::PosZ,
            chunk[ChunkIdx::new(coord.x, coord.y, coord.z + 1)],
        )
    {
        mesh_pos_z(
            b,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if registry.is_mesh_transparent(b, Direction::PosZ) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative z face
    if coord.z == 0
        || should_mesh_face(
            registry,
            b,
            Direction::NegZ,
            chunk[ChunkIdx::new(coord.x, coord.y, coord.z - 1)],
        )
    {
        mesh_neg_z(
            b,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if registry.is_mesh_transparent(b, Direction::NegZ) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //positive y face
    if coord.y == CHUNK_SIZE_U8 - 1
        || should_mesh_face(
            registry,
            b,
            Direction::PosY,
            chunk[ChunkIdx::new(coord.x, coord.y + 1, coord.z)],
        )
    {
        mesh_pos_y(
            b,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if registry.is_mesh_transparent(b, Direction::PosY) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative y face
    if coord.y == 0
        || should_mesh_face(
            registry,
            b,
            Direction::NegY,
            chunk[ChunkIdx::new(coord.x, coord.y - 1, coord.z)],
        )
    {
        mesh_neg_y(
            b,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if registry.is_mesh_transparent(b, Direction::NegY) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //positive x face
    if coord.x == CHUNK_SIZE_U8 - 1
        || should_mesh_face(
            registry,
            b,
            Direction::PosX,
            chunk[ChunkIdx::new(coord.x + 1, coord.y, coord.z)],
        )
    {
        mesh_pos_x(
            b,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if registry.is_mesh_transparent(b, Direction::PosX) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative x face
    if coord.x == 0
        || should_mesh_face(
            registry,
            b,
            Direction::NegX,
            chunk[ChunkIdx::new(coord.x - 1, coord.y, coord.z)],
        )
    {
        mesh_neg_x(
            b,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if registry.is_mesh_transparent(b, Direction::NegX) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
}
