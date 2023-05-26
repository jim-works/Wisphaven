use futures_lite::future;
use std::time::Instant;

use crate::world::chunk::*;
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

#[derive(Component)]
pub struct ChunkNeedsMesh {}

#[derive(Component)]
pub struct MeshTask {
    pub task: Task<MeshData>,
}

pub struct MeshData {
    verts: Vec<Vec3>,
    norms: Vec<Vec3>,
    tris: Vec<u32>,
    scale: f32,
}

#[derive(Resource)]
pub struct MeshTimer {
    pub timer: Timer
}

pub fn queue_meshing(
    query: Query<(Entity, &ChunkCoord), With<ChunkNeedsMesh>>,
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
    for (entity, coord) in query.iter() {
        if let Some(ctype) = level.chunks.get(coord) {
            if let ChunkType::Full(chunk) = ctype.value() {
                let mut neighbor_count = 0;
                let mut neighbors = [None, None, None, None, None, None];
                //i wish i could extrac this if let Some() shit into a function
                //but that makes the borrow checker angry
                for dir in Direction::iter() {
                    if let Some(ctype) = level.chunks.get(&coord.offset(dir)) {
                        if let ChunkType::Full(neighbor) = ctype.value() {
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
                        scale: 1.0
                    };
                    mesh_chunk(&meshing, &neighbors, &mut data);
                    data
                });
                commands
                    .entity(entity)
                    .remove::<ChunkNeedsMesh>()
                    .insert(MeshTask { task });
            }
        }
        let duration = Instant::now().duration_since(now).as_millis();
    }
    let duration = Instant::now().duration_since(now).as_millis();
    if len > 0 {
        println!(
            "queued mesh generation for {} chunks in {}ms",
            len, duration
        );
    }
}

pub fn poll_mesh_queue(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(Entity, &ChunkCoord, Option<&Handle<Mesh>>, &mut MeshTask)>,
) {
    //todo: parallelize this
    //(can't right now as Commands and StandardMaterial do not implement clone)
    let mut len = 0;
    for (entity, chunk, opt_mesh_handle, mut task) in query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            len += 1;
            if data.verts.len() > 0 {
                if let Some(mesh_handle) = opt_mesh_handle {
                    //update existing chunk
                    let mesh = meshes.get_mut(mesh_handle).unwrap();
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.verts);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.norms);
                    mesh.set_indices(Some(mesh::Indices::U32(data.tris)));
                } else {
                    //spawn new chunk
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.verts);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.norms);
                    mesh.set_indices(Some(mesh::Indices::U32(data.tris)));

                    commands.entity(entity).insert(PbrBundle {
                        mesh: meshes.add(mesh),
                        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
                        transform: Transform {
                            translation: chunk.to_vec3(),
                            ..default()
                        },
                        ..default()
                    });
                }
            }

            // mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 3]);
            commands.entity(entity).remove::<MeshTask>();
            if len > SPAWN_MESH_TIME_BUDGET_COUNT {
                break;
            }
        }
    }
    // let duration = Instant::now().duration_since(now).as_millis();
    // if len > 0 {
    //     println!("spawned {} chunk meshes in {}ms", len, duration);
    // }
}

fn mesh_chunk(chunk: &Chunk, neighbors: &[Option<Chunk>; 6], data: &mut MeshData) {
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        mesh_block(&chunk, neighbors, &chunk[i], ChunkIdx::from_usize(i), data);
    }
}

fn mesh_block(
    chunk: &Chunk,
    neighbors: &[Option<Chunk>; 6],
    b: &BlockType,
    coord: ChunkIdx,
    data: &mut MeshData,
) {
    if let BlockType::Empty = b {
        return;
    }
    let origin = coord.to_vec3();
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
fn mesh_neg_z(origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    data.verts.push(origin + Vec3::new(0., scale.y, 0.));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));
    data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
    data.verts.push(origin + Vec3::new(0., 0., 0.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
}
fn mesh_pos_z(origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    data.verts.push(origin + Vec3::new(0., 0., scale.z));
    data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, scale.z));
    data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
}

fn mesh_neg_x(origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    data.verts.push(origin + Vec3::new(0., 0., scale.z));
    data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
    data.verts.push(origin + Vec3::new(0., scale.y, 0.));
    data.verts.push(origin + Vec3::new(0., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
}

fn mesh_pos_x(origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, scale.z));
    data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
}

fn mesh_pos_y(origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    data.verts.push(origin + Vec3::new(0., scale.y, 0.));
    data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, scale.z));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
}

fn mesh_neg_y(origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    data.verts.push(origin + Vec3::new(0., 0., 0.));
    data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
    data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
    data.verts.push(origin + Vec3::new(0., 0., scale.z));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));
}

fn add_tris(tris: &mut Vec<u32>, first_vert_idx: u32) {
    tris.push(first_vert_idx);
    tris.push(first_vert_idx + 1);
    tris.push(first_vert_idx + 2);

    tris.push(first_vert_idx + 2);
    tris.push(first_vert_idx + 3);
    tris.push(first_vert_idx);
}
