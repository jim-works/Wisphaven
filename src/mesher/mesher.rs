use bevy::render::mesh::VertexAttributeValues;
use futures_lite::future;
use std::ops::Index;
use std::process::Output;
use std::time::Instant;

use crate::world::chunk::*;
use crate::worldgen::GeneratedChunk;
use crate::{
    util::Direction,
    world::{Level, *},
};
use bevy::{
    prelude::*,
    render::{mesh, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task},
};

use super::materials::ATTRIBUTE_AO;
use super::{SPAWN_MESH_TIME_BUDGET_COUNT, ChunkMaterial, ArrayTextureMaterial, materials::ATTRIBUTE_TEXLAYER};

#[derive(Component)]
pub struct NeedsMesh {}

#[derive(Component)]
pub struct MeshTask {
    pub task: Task<MeshData>,
}

pub struct MeshData {
    pub verts: Vec<Vec3>,
    pub norms: Vec<Vec3>,
    pub tris: Vec<u32>,
    pub uvs: Vec<Vec2>,
    pub layer_idx: Vec<i32>,
    pub ao_level: Vec<f32>,
    pub scale: f32,
    pub position: Vec3
}

#[derive(Resource)]
pub struct MeshTimer {
    pub timer: Timer
}

pub fn queue_meshing(
    query: Query<(Entity, &ChunkCoord), (With<GeneratedChunk>, With<NeedsMesh>)>,
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
        if let Some(ctype) = level.get_chunk(*coord) {
            if let ChunkType::Full(chunk) = ctype.value() {
                let mut neighbor_count = 0;
                let mut neighbors = [None, None, None, None, None, None];
                //i wish i could extrac this if let Some() shit into a function
                //but that makes the borrow checker angry
                for dir in Direction::iter() {
                    if let Some(ctype) = level.get_chunk(coord.offset(dir)) {
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
                        uvs: Vec::new(),
                        layer_idx: Vec::new(),
                        ao_level: Vec::new(),
                        scale: 1.0,
                        position: meshing.position.to_vec3()
                    };
                    mesh_chunk(&meshing, &neighbors, &mut data);
                    data
                });
                commands
                    .entity(entity)
                    .remove::<NeedsMesh>()
                    .insert(MeshTask { task });
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

pub fn poll_mesh_queue(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_material: Res<ChunkMaterial>,
    mut query: Query<(Entity, Option<&Handle<Mesh>>, &mut MeshTask)>,
) {
    if !chunk_material.loaded {
        warn!("polling mesh queue before chunk material is loaded!");
        return;
    }
    //todo: parallelize this
    //(can't right now as Commands and StandardMaterial do not implement clone)
    let mut len = 0;
    let now = Instant::now();
    for (entity, opt_mesh_handle, mut task) in query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            len += 1;
            if data.verts.len() > 0 {
                if let Some(mesh_handle) = opt_mesh_handle {
                    //update existing chunk
                    let mesh = meshes.get_mut(mesh_handle).unwrap();
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.verts);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.norms);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs);
                    
                    mesh.insert_attribute(ATTRIBUTE_TEXLAYER, data.layer_idx);
                    mesh.insert_attribute(ATTRIBUTE_AO, data.ao_level);
                    mesh.set_indices(Some(mesh::Indices::U32(data.tris)));
                } else {
                    //spawn new chunk
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.verts);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.norms);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs);
                    mesh.insert_attribute(ATTRIBUTE_TEXLAYER, data.layer_idx);
                    mesh.insert_attribute(ATTRIBUTE_AO, data.ao_level);

                    mesh.set_indices(Some(mesh::Indices::U32(data.tris)));
                    commands.entity(entity).insert(MaterialMeshBundle::<ArrayTextureMaterial> {
                        mesh: meshes.add(mesh),
                        //just cloning the handle, is it worth matching then cloning weak?
                        material: chunk_material.opaque_material.clone().unwrap(),
                        transform: Transform {
                            translation: data.position,
                            ..default()
                        },
                        ..default()
                    });
                }
            } else if let Some(_) = opt_mesh_handle  {
                //remove old mesh from existing chunk if the new mesh is empty
                commands.entity(entity).remove::<Handle<Mesh>>();
            }
            commands.entity(entity).remove::<MeshTask>();
            if len > SPAWN_MESH_TIME_BUDGET_COUNT {
                break;
            }
        }
    }
    let duration = Instant::now().duration_since(now).as_millis();
    if len > 0 {
        println!("spawned {} chunk meshes in {}ms", len, duration);
    }
}

fn mesh_chunk(chunk: &Chunk, neighbors: &[Option<Chunk>; 6], data: &mut MeshData) {
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        let coord = ChunkIdx::from_usize(i);
        mesh_block(&chunk, neighbors, &chunk[i], coord, coord.to_vec3()*data.scale, data);
    }
}

fn mesh_block(
    chunk: &Chunk,
    neighbors: &[Option<Chunk>; 6],
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
            mesh_pos_z(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y, coord.z + 1)],
        BlockType::Empty
    ) {
        mesh_pos_z(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
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
            mesh_neg_z(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y, coord.z - 1)],
        BlockType::Empty
    ) {
        mesh_neg_z(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //positive y face
    if coord.y == CHUNK_SIZE_U8 - 1 {
        if match &neighbors[Direction::PosY.to_idx()] {
            Some(c) => matches!(c[ChunkIdx::new(coord.x, 0, coord.z)], BlockType::Empty),
            _ => true,
        } {
            mesh_pos_y(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y + 1, coord.z)],
        BlockType::Empty
    ) {
        mesh_pos_y(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
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
            mesh_neg_y(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x, coord.y - 1, coord.z)],
        BlockType::Empty
    ) {
        mesh_neg_y(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
    //positive x face
    if coord.x == CHUNK_SIZE_U8 - 1 {
        if match &neighbors[Direction::PosX.to_idx()] {
            Some(c) => matches!(c[ChunkIdx::new(0, coord.y, coord.z)], BlockType::Empty),
            _ => true,
        } {
            mesh_pos_x(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x + 1, coord.y, coord.z)],
        BlockType::Empty
    ) {
        mesh_pos_x(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
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
            mesh_neg_x(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
        }
    } else if matches!(
        chunk[ChunkIdx::new(coord.x - 1, coord.y, coord.z)],
        BlockType::Empty
    ) {
        mesh_neg_x(b, chunk, coord, origin, Vec3::new(data.scale,data.scale,data.scale), data);
    }
}
pub fn mesh_neg_z(b: &BlockType, chunk: &impl Index<ChunkIdx, Output=BlockType>, coord: ChunkIdx, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    match b {
        BlockType::Basic(id) => {
            let x = *id;
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, false, false, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, true, false, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, true, false, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, false, false, data);
        },
        _ => {}
    }
    data.verts.push(origin + Vec3::new(0., 0., 0.));
    data.verts.push(origin + Vec3::new(0., scale.y, 0.));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));
    data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.uvs.push(Vec2::new(1.0,1.0));
    data.uvs.push(Vec2::new(1.0,0.0));
    data.uvs.push(Vec2::new(0.0,0.0));
    data.uvs.push(Vec2::new(0.0,1.0));
}
pub fn mesh_pos_z(b: &BlockType, chunk: &impl Index<ChunkIdx, Output=BlockType>, coord: ChunkIdx, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    //add_uvs(&mut data.uvs);
    match b {
        BlockType::Basic(id) => {
            let x = *id;
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, false, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, false, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, true, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, true, true, data);
        },
        _ => {}
    }
    data.verts.push(origin + Vec3::new(0., 0., scale.z));
    data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, scale.z));
    data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.uvs.push(Vec2::new(0.0,1.0));
    data.uvs.push(Vec2::new(1.0,1.0));
    data.uvs.push(Vec2::new(1.0,0.0));
    data.uvs.push(Vec2::new(0.0,0.0));
    
}

pub fn mesh_neg_x(b: &BlockType, chunk: &impl Index<ChunkIdx, Output=BlockType>, coord: ChunkIdx, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    match b {
        BlockType::Basic(id) => {
            let x = *id;
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, false, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, true, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, true, false, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, false, false, data);
        },
        _ => {}
    }
    data.verts.push(origin + Vec3::new(0., 0., scale.z));
    data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
    data.verts.push(origin + Vec3::new(0., scale.y, 0.));
    data.verts.push(origin + Vec3::new(0., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.uvs.push(Vec2::new(1.0,1.0));
    data.uvs.push(Vec2::new(1.0,0.0));
    data.uvs.push(Vec2::new(0.0,0.0));
    data.uvs.push(Vec2::new(0.0,1.0));
}

pub fn mesh_pos_x(b: &BlockType, chunk: &impl Index<ChunkIdx, Output=BlockType>, coord: ChunkIdx, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    match b {
        BlockType::Basic(id) => {
            let x = *id;
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, true, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, false, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, false, false, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, true, false, data);
        },
        _ => {}
    }
    data.verts.push(origin + Vec3::new(scale.x, scale.y, scale.z));
    data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
    data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));
    
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
    data.uvs.push(Vec2::new(0.0,0.0));
    data.uvs.push(Vec2::new(0.0,1.0));
    data.uvs.push(Vec2::new(1.0,1.0));
    data.uvs.push(Vec2::new(1.0,0.0));
}

pub fn mesh_pos_y(b: &BlockType, chunk: &impl Index<ChunkIdx, Output=BlockType>, coord: ChunkIdx, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    match b {
        BlockType::Basic(id) => {
            let x = *id;
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, true, false, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, true, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, true, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, true, false, data);
        },
        _ => {}
    }
    data.verts.push(origin + Vec3::new(0., scale.y, 0.));
    data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, scale.z));
    data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.uvs.push(Vec2::new(1.0,1.0));
    data.uvs.push(Vec2::new(1.0,0.0));
    data.uvs.push(Vec2::new(0.0,0.0));
    data.uvs.push(Vec2::new(0.0,1.0));
}

pub fn mesh_neg_y(b: &BlockType, chunk: &impl Index<ChunkIdx, Output=BlockType>, coord: ChunkIdx, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    match b {
        BlockType::Basic(id) => {
            let x = *id;
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, false, false, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, false, false, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, true, false, true, data);
            data.layer_idx.push(x as i32);
            add_ao(chunk, coord, false, false, true, data);
        },
        _ => {}
    }
    data.verts.push(origin + Vec3::new(0., 0., 0.));
    data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
    data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
    data.verts.push(origin + Vec3::new(0., 0., scale.z));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.uvs.push(Vec2::new(1.0,1.0));
    data.uvs.push(Vec2::new(0.0,1.0));
    data.uvs.push(Vec2::new(0.0,0.0));
    data.uvs.push(Vec2::new(1.0,0.0));
}

fn add_tris(tris: &mut Vec<u32>, first_vert_idx: u32) {
    tris.push(first_vert_idx);
    tris.push(first_vert_idx + 1);
    tris.push(first_vert_idx + 2);

    tris.push(first_vert_idx + 2);
    tris.push(first_vert_idx + 3);
    tris.push(first_vert_idx);
}

//TODO: add support for chunk neighbors
//https://0fps.net/2013/07/03/ambient-occlusion-for-minecraft-like-worlds/
fn add_ao(
    chunk: &impl Index<ChunkIdx, Output=BlockType>,
    //neighbors: &[Option<Chunk>; 6],
    coord: ChunkIdx,
    pos_x: bool,
    pos_y: bool,
    pos_z: bool,
    data: &mut MeshData
) {
    let side1_coord = IVec3::new(coord.x as i32 + if pos_x {1} else {-1}, coord.y as i32 + if pos_y {1} else {-1}, coord.z as i32);
    let side2_coord = IVec3::new(coord.x as i32, coord.y as i32 + if pos_y {1} else {-1}, coord.z as i32 + if pos_z {1} else {-1});
    let corner_coord = IVec3::new(coord.x as i32 + if pos_x {1} else {-1}, coord.y as i32 + if pos_y {1} else {-1}, coord.z as i32 + if pos_z {1} else {-1});
    let mut side1 = false;
    let mut side2 = false;
    let mut corner = false;

    if side1_coord.x < CHUNK_SIZE_I32 && side1_coord.x >= 0 {
        if side1_coord.y < CHUNK_SIZE_I32 && side1_coord.y >= 0 {
            side1 = matches!(chunk[ChunkIdx::new(side1_coord.x as u8, side1_coord.y as u8, side1_coord.z as u8)], BlockType::Basic(_));
        }
    }
    if side2_coord.z < CHUNK_SIZE_I32 && side2_coord.z >= 0 {
        if side2_coord.y < CHUNK_SIZE_I32 && side2_coord.y >= 0 {
            side2 = matches!(chunk[ChunkIdx::new(side2_coord.x as u8, side2_coord.y as u8, side2_coord.z as u8)], BlockType::Basic(_));
        }
    }
    if corner_coord.x < CHUNK_SIZE_I32 && corner_coord.x >= 0 {
        if corner_coord.y < CHUNK_SIZE_I32 && corner_coord.y >= 0 {
            if corner_coord.z < CHUNK_SIZE_I32 && corner_coord.z >= 0 {
                corner = matches!(chunk[ChunkIdx::new(corner_coord.x as u8, corner_coord.y as u8, corner_coord.z as u8)], BlockType::Basic(_));
            }
        }
    }

    data.ao_level.push(neighbors_to_ao(side1, side2, corner));
}

//calculates ao level based on neighbor count
//each argument is 1 if that neighbor is present, 0 otherwise
//https://0fps.net/2013/07/03/ambient-occlusion-for-minecraft-like-worlds/
fn neighbors_to_ao(side1: bool, side2: bool, corner: bool) -> f32 {
    const LEVEL_FOR_NEIGHBORS: [f32; 4] = [0.5, 0.7, 0.9, 1.0];
    if side1 && side2 {
        return LEVEL_FOR_NEIGHBORS[0];
    }
    return LEVEL_FOR_NEIGHBORS[3-(side1 as usize+side2 as usize +corner as usize)];
}
