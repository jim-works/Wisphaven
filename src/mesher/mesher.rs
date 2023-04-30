use std::time::Instant;

use crate::chunk::{*,chunk::*};
use bevy::{
    prelude::*,
    render::{mesh, render_resource::PrimitiveTopology},
};

#[derive(Component)]
pub struct ChunkNeedsMesh {}

pub fn mesh_new(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(Entity, &Chunk, Option<&Handle<Mesh>>), With<ChunkNeedsMesh>>,
) {
    let now = Instant::now();
    for (entity, chunk, opt_mesh_handle) in query.iter() {
        
        let mut verts: Vec<Vec3> = Vec::new();
        let mut norms: Vec<Vec3> = Vec::new();
        let mut tris: Vec<u32> = Vec::new();

        mesh_chunk(chunk, &mut verts, &mut norms, &mut tris);

        if let Some(mesh_handle) = opt_mesh_handle {
            //update existing chunk
            let mesh = meshes.get_mut(mesh_handle).unwrap();
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, norms);
            mesh.set_indices(Some(mesh::Indices::U32(tris)));
        } else {
            //spawn new chunk
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, norms);
            mesh.set_indices(Some(mesh::Indices::U32(tris)));

            commands.entity(entity).insert(PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
                transform: Transform {
                    translation: chunk.position.to_vec3(),
                    ..default()
                },
                ..default()
            });
            println!("at {:?}", chunk.position.to_vec3());
        }
        
        // mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 3]);
        commands.entity(entity).remove::<ChunkNeedsMesh>();
    }
    let duration = Instant::now().duration_since(now).as_millis();
    if duration > 0 {println!("finished meshing in {}ms", duration);}
}

fn mesh_chunk (
    chunk: &Chunk,
    verts: &mut Vec<Vec3>,
    norms: &mut Vec<Vec3>,
    tris: &mut Vec<u32>,
) {
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        mesh_block(
            &chunk,
            &chunk[i],
            ChunkIdx::from_usize(i),
            verts,
            norms,
            tris
        );
    }
}

fn mesh_block(
    chunk: &Chunk,
    b: &BlockType,
    coord: ChunkIdx,
    verts: &mut Vec<Vec3>,
    norms: &mut Vec<Vec3>,
    tris: &mut Vec<u32>,
) {
    if let BlockType::Empty = b {
        return;
    }
    let origin = coord.to_vec3();
    if coord.z == CHUNK_SIZE_U8-1 || matches!(chunk[ChunkIdx::new(coord.x,coord.y,coord.z+1)], BlockType::Empty) {
        mesh_pos_z(origin, verts, norms, tris);
    }
    if coord.z == 0 || matches!(chunk[ChunkIdx::new(coord.x,coord.y,coord.z-1)], BlockType::Empty) {
        mesh_neg_z(origin, verts, norms, tris);
    }

    if coord.y == CHUNK_SIZE_U8-1 || matches!(chunk[ChunkIdx::new(coord.x,coord.y+1,coord.z)], BlockType::Empty) {
        mesh_pos_y(origin, verts, norms, tris);
    }
    if coord.y == 0 || matches!(chunk[ChunkIdx::new(coord.x,coord.y-1,coord.z)], BlockType::Empty) {
        mesh_neg_y(origin, verts, norms, tris);
    } 

    if coord.x == CHUNK_SIZE_U8-1 || matches!(chunk[ChunkIdx::new(coord.x+1,coord.y,coord.z)], BlockType::Empty) {
        mesh_pos_x(origin, verts, norms, tris);
    }
    if coord.x == 0 || matches!(chunk[ChunkIdx::new(coord.x-1,coord.y,coord.z)], BlockType::Empty) {
        mesh_neg_x(origin, verts, norms, tris);
    }
}
fn mesh_neg_z(origin: Vec3, verts: &mut Vec<Vec3>, norms: &mut Vec<Vec3>, tris: &mut Vec<u32>) {
    add_tris(tris, verts.len() as u32);
    verts.push(origin + Vec3::new(0., 1., 0.));
    verts.push(origin + Vec3::new(1., 1., 0.));
    verts.push(origin + Vec3::new(1., 0., 0.));
    verts.push(origin + Vec3::new(0., 0., 0.));
    norms.push(Vec3::new(0., 0., -1.));
    norms.push(Vec3::new(0., 0., -1.));
    norms.push(Vec3::new(0., 0., -1.));
    norms.push(Vec3::new(0., 0., -1.));
}
fn mesh_pos_z(origin: Vec3, verts: &mut Vec<Vec3>, norms: &mut Vec<Vec3>, tris: &mut Vec<u32>) {
    add_tris(tris, verts.len() as u32);
    verts.push(origin + Vec3::new(0., 0., 1.));
    verts.push(origin + Vec3::new(1., 0., 1.));
    verts.push(origin + Vec3::new(1., 1., 1.));
    verts.push(origin + Vec3::new(0., 1., 1.));
    norms.push(Vec3::new(0., 0., 1.));
    norms.push(Vec3::new(0., 0., 1.));
    norms.push(Vec3::new(0., 0., 1.));
    norms.push(Vec3::new(0., 0., 1.));
}

fn mesh_neg_x(origin: Vec3, verts: &mut Vec<Vec3>, norms: &mut Vec<Vec3>, tris: &mut Vec<u32>) {
    add_tris(tris, verts.len() as u32);
    verts.push(origin + Vec3::new(0., 0., 1.));
    verts.push(origin + Vec3::new(0., 1., 1.));
    verts.push(origin + Vec3::new(0., 1., 0.));
    verts.push(origin + Vec3::new(0., 0., 0.));
    norms.push(Vec3::new(-1., 0., 0.));
    norms.push(Vec3::new(-1., 0., 0.));
    norms.push(Vec3::new(-1., 0., 0.));
    norms.push(Vec3::new(-1., 0., 0.));
}

fn mesh_pos_x(origin: Vec3, verts: &mut Vec<Vec3>, norms: &mut Vec<Vec3>, tris: &mut Vec<u32>) {
    add_tris(tris, verts.len() as u32);
    verts.push(origin + Vec3::new(1., 0., 0.));
    verts.push(origin + Vec3::new(1., 1., 0.));
    verts.push(origin + Vec3::new(1., 1., 1.));
    verts.push(origin + Vec3::new(1., 0., 1.));
    norms.push(Vec3::new(1., 0., 0.));
    norms.push(Vec3::new(1., 0., 0.));
    norms.push(Vec3::new(1., 0., 0.));
    norms.push(Vec3::new(1., 0., 0.));
}

fn mesh_pos_y(origin: Vec3, verts: &mut Vec<Vec3>, norms: &mut Vec<Vec3>, tris: &mut Vec<u32>) {
    add_tris(tris, verts.len() as u32);
    verts.push(origin + Vec3::new(0., 1., 0.));
    verts.push(origin + Vec3::new(0., 1., 1.));
    verts.push(origin + Vec3::new(1., 1., 1.));
    verts.push(origin + Vec3::new(1., 1., 0.));
    norms.push(Vec3::new(0., 1., 0.));
    norms.push(Vec3::new(0., 1., 0.));
    norms.push(Vec3::new(0., 1., 0.));
    norms.push(Vec3::new(0., 1., 0.));
}

fn mesh_neg_y(origin: Vec3, verts: &mut Vec<Vec3>, norms: &mut Vec<Vec3>, tris: &mut Vec<u32>) {
    add_tris(tris, verts.len() as u32);
    verts.push(origin + Vec3::new(0., 0., 0.));
    verts.push(origin + Vec3::new(1., 0., 0.));
    verts.push(origin + Vec3::new(1., 0., 1.));
    verts.push(origin + Vec3::new(0., 0., 1.));
    norms.push(Vec3::new(0., -1., 0.));
    norms.push(Vec3::new(0., -1., 0.));
    norms.push(Vec3::new(0., -1., 0.));
    norms.push(Vec3::new(0., -1., 0.));
}

fn add_tris(tris: &mut Vec<u32>, first_vert_idx: u32) {
    tris.push(first_vert_idx);
    tris.push(first_vert_idx + 1);
    tris.push(first_vert_idx + 2);

    
    tris.push(first_vert_idx + 2);
    tris.push(first_vert_idx + 3);
    tris.push(first_vert_idx);
}
