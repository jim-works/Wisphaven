use super::generator::*;

use crate::util::Direction;
use crate::world::chunk::*;
use crate::world::{Level, *};
use crate::worldgen::GeneratedLODChunk;
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

#[derive(Resource)]
pub struct LODMeshTimer {
    pub timer: Timer,
}

//there may be a cleaner way to do this, with some traits
//but I expect there will be significant differences between LOD and non-LOD meshing, so probably best to have separate functions entirely
pub fn queue_meshing_lod(
    query: Query<(Entity, &ChunkCoord, &LODLevel), (With<GeneratedLODChunk>, With<NeedsMesh>)>,
    level: Res<Level>,
    time: Res<Time>,
    mut timer: ResMut<LODMeshTimer>,
    commands: ParallelCommands,
    mesh_query: Query<&BlockMesh>
) {
    let _my_span = info_span!("queue_meshing_lod", name = "queue_meshing_lod").entered();
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }
    let pool = AsyncComputeTaskPool::get();
    query.par_iter().for_each(|(entity, coord, lod)| {
        if let Some(chunks) = level.get_lod_chunks(lod.level.into()) {
            if let Some(ctype) = chunks.get(coord) {
                if let LODChunkType::Full(chunk) = ctype.value() {
                    //don't wait for neighbors since we won't have all neighbors generated, as there is a big hole in the middle of where we generate
                    //TODO: greedy meshing is very important here
                    let meshing = chunk.with_storage(Box::new(chunk.blocks.get_components(&mesh_query)));
                    let task = pool.spawn(async move {
                        let mut data = ChunkMesh::new(meshing.scale() as f32);
                        mesh_chunk_lod(&meshing, &mut data);
                        data
                    });
                    commands.command_scope(|mut commands| {
                        commands.entity(entity)
                        .remove::<NeedsMesh>()
                        .insert(MeshTask { task });
                    }); 
                }
            }
        }
    });
}

fn mesh_chunk_lod<T: ChunkStorage<BlockMesh>>(chunk: &Chunk<T, BlockMesh>, data: &mut ChunkMesh) {
    let _my_span = info_span!("mesh_chunk_lod", name = "mesh_chunk_lod").entered();
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        let coord = ChunkIdx::from_usize(i);
        mesh_block_lod(chunk,&chunk[i],coord,coord.to_vec3() * data.scale,data);
    }
}

fn mesh_block_lod<T: ChunkStorage<BlockMesh>>(
    chunk: &Chunk<T, BlockMesh>,
    b: &BlockMesh,
    coord: ChunkIdx,
    origin: Vec3,
    data: &mut ChunkMesh,
) {
    if matches!(b.shape, BlockMeshShape::Empty) {
        return;
    }
    if coord.z == CHUNK_SIZE_U8 - 1
        || should_mesh_face(
            b,
            Direction::PosZ,
            &chunk[ChunkIdx::new(coord.x, coord.y, coord.z + 1)],
        )
    {
        mesh_pos_z(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.shape.is_transparent(Direction::PosZ) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative z face
    if coord.z == 0
        || should_mesh_face(
            b,
            Direction::NegZ,
            &chunk[ChunkIdx::new(coord.x, coord.y, coord.z - 1)],
        )
    {
        mesh_neg_z(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.shape.is_transparent(Direction::NegZ) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //positive y face
    if coord.y == CHUNK_SIZE_U8 - 1
        || should_mesh_face(
            b,
            Direction::PosY,
            &chunk[ChunkIdx::new(coord.x, coord.y + 1, coord.z)],
        )
    {
        mesh_pos_y(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.shape.is_transparent(Direction::PosY) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative y face
    if coord.y == 0
        || should_mesh_face(
            b,
            Direction::NegY,
            &chunk[ChunkIdx::new(coord.x, coord.y - 1, coord.z)],
        )
    {
        mesh_neg_y(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.shape.is_transparent(Direction::NegY) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //positive x face
    if coord.x == CHUNK_SIZE_U8 - 1
        || should_mesh_face(
            b,
            Direction::PosX,
            &chunk[ChunkIdx::new(coord.x + 1, coord.y, coord.z)],
        )
    {
        mesh_pos_x(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.shape.is_transparent(Direction::PosX) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative x face
    if coord.x == 0
        || should_mesh_face(
            b,
            Direction::NegX,
            &chunk[ChunkIdx::new(coord.x - 1, coord.y, coord.z)],
        )
    {
        mesh_neg_x(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.shape.is_transparent(Direction::NegX) {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
}
