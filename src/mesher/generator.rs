use bevy::pbr::NotShadowCaster;
use futures_lite::future;
use std::ops::Index;
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
use super::{
    materials::ATTRIBUTE_TEXLAYER, ArrayTextureMaterial, ChunkMaterial,
    SPAWN_MESH_TIME_BUDGET_COUNT,
};

#[derive(Component)]
pub struct NeedsMesh;

#[derive(Component)]
pub struct MeshTask {
    pub task: Task<ChunkMesh>,
}

pub struct ChunkMesh {
    pub opaque: MeshData,
    pub transparent: MeshData,
    pub scale: f32,
}

impl ChunkMesh {
    pub fn new(scale: f32) -> Self {
        Self {
            opaque: MeshData::new(),
            transparent: MeshData::new(),
            scale,
        }
    }
}

pub struct MeshData {
    pub verts: Vec<Vec3>,
    pub norms: Vec<Vec3>,
    pub tris: Vec<u32>,
    pub uvs: Vec<Vec2>,
    pub layer_idx: Vec<i32>,
    pub ao_level: Vec<f32>,
}

impl MeshData {
    pub fn new() -> Self {
        Self {
            verts: Vec::new(),
            norms: Vec::new(),
            tris: Vec::new(),
            uvs: Vec::new(),
            layer_idx: Vec::new(),
            ao_level: Vec::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.verts.is_empty()
    }
}

#[derive(Component)]
pub struct ChunkMeshChild;

#[derive(Resource)]
pub struct MeshTimer {
    pub timer: Timer,
}

const SQRT_2_4: f32 = 0.353553390593; //sqrt(2)/4

pub fn queue_meshing(
    query: Query<(Entity, &ChunkCoord), (With<GeneratedChunk>, With<NeedsMesh>)>,
    level: Res<Level>,
    time: Res<Time>,
    mesh_query: Query<&BlockMesh>,
    mut timer: ResMut<MeshTimer>,
    commands: ParallelCommands,
) {
    let _my_span = info_span!("queue_meshing", name = "queue_meshing").entered();
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }
    let pool = AsyncComputeTaskPool::get();
    query.par_iter().for_each(|(entity, coord)| {
        if let Some(ctype) = level.get_chunk(*coord) {
            if let ChunkType::Full(chunk) = ctype.value() {
                let mut neighbor_count = 0;
                let mut neighbors = [None, None, None, None, None, None];
                //i wish i could extrac this if let Some() shit into a function
                //but that makes the borrow checker angry
                //check neighbor count first becuase Chunk::get_components is a very expensive operation
                for dir in Direction::iter() {
                    if let Some(ctype) = level.get_chunk(coord.offset(dir)) {
                        if let ChunkType::Full(_) = ctype.value() {
                            neighbor_count += 1;
                        }
                    }
                }
                if neighbor_count != 6 {
                    //don't mesh if all neighbors aren't ready yet
                    return;
                }
                //we have to check neighbor counts again because a chunk could be removed since the last loop, and we don't clone anything before
                neighbor_count = 0;
                for dir in Direction::iter() {
                    if let Some(ctype) = level.get_chunk(coord.offset(dir)) {
                        if let ChunkType::Full(neighbor) = ctype.value() {
                            neighbor_count += 1;
                            neighbors[dir.to_idx()] = Some(neighbor.with_storage(Box::new(neighbor.blocks.get_components::<BlockMesh>(&mesh_query))));
                        }
                    }
                }
                if neighbor_count != 6 {
                    //don't mesh if all neighbors aren't ready yet
                    return;
                }
                let meshing = chunk.with_storage(Box::new(chunk.blocks.get_components(&mesh_query)));
                let task = pool.spawn(async move {
                    let mut data = ChunkMesh::new(1.0);
                    mesh_chunk(&meshing, &neighbors, &mut data);
                    data
                });
                commands.command_scope(|mut commands| {
                    commands.entity(entity)
                    .remove::<NeedsMesh>()
                    .insert(MeshTask { task });
                }); 
            }
        }
    });

}
pub fn poll_mesh_queue(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_material: Res<ChunkMaterial>,
    mut query: Query<(Entity, &mut MeshTask, Option<&Children>)>,
    children_query: Query<Entity, With<ChunkMeshChild>>,
) {
    let _my_span = info_span!("poll_mesh_queue", name = "poll_mesh_queue").entered();
    if !chunk_material.loaded {
        warn!("polling mesh queue before chunk material is loaded!");
        return;
    }
    let mut len = 0;
    let now = Instant::now();
    for (entity, mut task, opt_children) in query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            len += 1;
            //remove old mesh
            if let Some(children) = opt_children {
                for child in children {
                    if children_query.contains(*child) {
                        commands.entity(*child).despawn_recursive();
                    }
                }
            }
            //add new meshes
            if !data.opaque.is_empty() {
                spawn_mesh::<NotShadowCaster>(
                    data.opaque,
                    chunk_material.opaque_material.clone().unwrap(),
                    &mut commands,
                    &mut meshes,
                    None,
                    entity,
                );
            }
            if !data.transparent.is_empty() {
                spawn_mesh(
                    data.transparent,
                    chunk_material.transparent_material.clone().unwrap(),
                    &mut commands,
                    &mut meshes,
                    Some(NotShadowCaster), //todo: fix this for transparent materials (think I need to modify prepass shader and get the vertex info in there somehow)
                    entity,
                );
            }

            commands.entity(entity).remove::<MeshTask>();
            if len > SPAWN_MESH_TIME_BUDGET_COUNT {
                break;
            }
        }
    }
    let duration = Instant::now().duration_since(now).as_millis();
    if len > 0 {
        debug!("spawned {} chunk meshes in {}ms", len, duration);
    }
}

pub fn spawn_mesh<T: Bundle>(
    data: MeshData,
    material: Handle<ArrayTextureMaterial>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    components: Option<T>,
    entity: Entity,
) {
    //spawn new mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.verts);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.norms);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs);
    mesh.insert_attribute(ATTRIBUTE_TEXLAYER, data.layer_idx);
    mesh.insert_attribute(ATTRIBUTE_AO, data.ao_level);

    mesh.set_indices(Some(mesh::Indices::U32(data.tris)));
    commands.entity(entity).with_children(|children| {
        let mut ec = children.spawn((
            MaterialMeshBundle::<ArrayTextureMaterial> {
                mesh: meshes.add(mesh),
                material,
                transform: Transform::default(),
                ..default()
            },
            ChunkMeshChild,
        ));
        if let Some(bundle) = components {
            ec.insert(bundle);
        }
    });
}

fn mesh_chunk<T: ChunkStorage<BlockMesh>>(
    chunk: &Chunk<T,BlockMesh>,
    neighbors: &[Option<Chunk<T,BlockMesh>>; 6],
    data: &mut ChunkMesh,
) {
    let _my_span = info_span!("mesh_chunk", name = "mesh_chunk").entered();
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        let coord = ChunkIdx::from_usize(i);
        mesh_block(
                chunk,
                neighbors,
                &chunk[i],
                coord,
                coord.to_vec3() * data.scale,
                data
        );
    }
}
pub fn should_mesh_face(
    block: &BlockMesh,
    block_face: Direction,
    neighbor: &BlockMesh,
) -> bool {
    if !block.use_transparent_shader && neighbor.use_transparent_shader {
        return true;
    }
    match block.shape {
        BlockMeshShape::Uniform(_) | BlockMeshShape::MultiTexture(_) => {
            block != neighbor && neighbor.shape.is_transparent(block_face.opposite())
        },
        BlockMeshShape::BottomSlab(_, _) => {
            block_face == Direction::PosY
                || block != neighbor && neighbor.shape.is_transparent(block_face.opposite())
        },
        BlockMeshShape::Cross(_) => !matches!(block_face, Direction::PosY | Direction::NegY),
        BlockMeshShape::Empty => false,

    }
}
fn mesh_block<T: ChunkStorage<BlockMesh>>(
    chunk: &Chunk<T,BlockMesh>,
    neighbors: &[Option<Chunk<T,BlockMesh>>; 6],
    b: &BlockMesh,
    coord: ChunkIdx,
    origin: Vec3,
    data: &mut ChunkMesh,
) {
    if matches!(b.shape, BlockMeshShape::Empty) {
        return;
    }
    if coord.z == CHUNK_SIZE_U8 - 1 {
        if match &neighbors[Direction::PosZ.to_idx()] {
            Some(c) => should_mesh_face(
                b,
                Direction::PosZ,
                &c[ChunkIdx::new(coord.x, coord.y, 0)],
            ),
            _ => true,
        } {
            mesh_pos_z(
                &b.shape,
                chunk,
                coord,
                origin,
                Vec3::new(data.scale, data.scale, data.scale),
                if b.use_transparent_shader {
                    &mut data.transparent
                } else {
                    &mut data.opaque
                },
            );
        }
    } else if should_mesh_face(
        b,
        Direction::PosZ,
        &chunk[ChunkIdx::new(coord.x, coord.y, coord.z + 1)],
    ) {
        mesh_pos_z(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.use_transparent_shader {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative z face
    if coord.z == 0 {
        if match &neighbors[Direction::NegZ.to_idx()] {
            Some(c) => should_mesh_face(
                b,
                Direction::NegZ,
                &c[ChunkIdx::new(coord.x, coord.y, CHUNK_SIZE_U8 - 1)],
            ),
            _ => true,
        } {
            mesh_neg_z(
                &b.shape,
                chunk,
                coord,
                origin,
                Vec3::new(data.scale, data.scale, data.scale),
                if b.use_transparent_shader {
                    &mut data.transparent
                } else {
                    &mut data.opaque
                },
            );
        }
    } else if should_mesh_face(
        b,
        Direction::NegZ,
        &chunk[ChunkIdx::new(coord.x, coord.y, coord.z - 1)],
    ) {
        mesh_neg_z(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.use_transparent_shader {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //positive y face
    if coord.y == CHUNK_SIZE_U8 - 1 {
        if match &neighbors[Direction::PosY.to_idx()] {
            Some(c) => should_mesh_face(
                b,
                Direction::PosY,
                &c[ChunkIdx::new(coord.x, 0, coord.z)],
            ),
            _ => true,
        } {
            mesh_pos_y(
                &b.shape,
                chunk,
                coord,
                origin,
                Vec3::new(data.scale, data.scale, data.scale),
                if b.use_transparent_shader {
                    &mut data.transparent
                } else {
                    &mut data.opaque
                },
            );
        }
    } else if should_mesh_face(
        b,
        Direction::PosY,
        &chunk[ChunkIdx::new(coord.x, coord.y + 1, coord.z)],
    ) {
        mesh_pos_y(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.use_transparent_shader {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative y face
    if coord.y == 0 {
        if match &neighbors[Direction::NegY.to_idx()] {
            Some(c) => should_mesh_face(
                b,
                Direction::NegY,
                &c[ChunkIdx::new(coord.x, CHUNK_SIZE_U8 - 1, coord.z)],
            ),
            _ => true,
        } {
            mesh_neg_y(
                &b.shape,
                chunk,
                coord,
                origin,
                Vec3::new(data.scale, data.scale, data.scale),
                if b.use_transparent_shader {
                    &mut data.transparent
                } else {
                    &mut data.opaque
                },
            );
        }
    } else if should_mesh_face(
        b,
        Direction::NegY,
        &chunk[ChunkIdx::new(coord.x, coord.y - 1, coord.z)],
    ) {
        mesh_neg_y(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.use_transparent_shader {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //positive x face
    if coord.x == CHUNK_SIZE_U8 - 1 {
        if match &neighbors[Direction::PosX.to_idx()] {
            Some(c) => should_mesh_face(
                b,
                Direction::PosX,
                &c[ChunkIdx::new(0, coord.y, coord.z)],
            ),
            _ => true,
        } {
            mesh_pos_x(
                &b.shape,
                chunk,
                coord,
                origin,
                Vec3::new(data.scale, data.scale, data.scale),
                if b.use_transparent_shader {
                    &mut data.transparent
                } else {
                    &mut data.opaque
                },
            );
        }
    } else if should_mesh_face(
        b,
        Direction::PosX,
        &chunk[ChunkIdx::new(coord.x + 1, coord.y, coord.z)],
    ) {
        mesh_pos_x(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.use_transparent_shader {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
    //negative x face
    if coord.x == 0 {
        if match &neighbors[Direction::NegX.to_idx()] {
            Some(c) => should_mesh_face(
                b,
                Direction::NegX,
                &c[ChunkIdx::new(CHUNK_SIZE_U8 - 1, coord.y, coord.z)],
            ),
            _ => true,
        } {
            mesh_neg_x(
                &b.shape,
                chunk,
                coord,
                origin,
                Vec3::new(data.scale, data.scale, data.scale),
                if b.use_transparent_shader {
                    &mut data.transparent
                } else {
                    &mut data.opaque
                },
            );
        }
    } else if should_mesh_face(
        b,
        Direction::NegX,
        &chunk[ChunkIdx::new(coord.x - 1, coord.y, coord.z)],
    ) {
        mesh_neg_x(
            &b.shape,
            chunk,
            coord,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            if b.use_transparent_shader {
                &mut data.transparent
            } else {
                &mut data.opaque
            },
        );
    }
}

//TODO: Set uv scale for repeating textures
pub fn mesh_neg_z(
    b: &BlockMeshShape,
    chunk: &impl Index<ChunkIdx, Output = BlockMesh>,
    coord: ChunkIdx,
    origin: Vec3,
    scale: Vec3,
    data: &mut MeshData,
) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, true, true, false, data);
            add_ao(chunk, coord, true, false, false, data);

            data.verts.push(origin + Vec3::new(0., 0., 0.));
            data.verts.push(origin + Vec3::new(0., scale.y, 0.));
            data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        }
        BlockMeshShape::MultiTexture(tex) => {
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, true, true, false, data);
            add_ao(chunk, coord, true, false, false, data);

            data.verts.push(origin + Vec3::new(0., 0., 0.));
            data.verts.push(origin + Vec3::new(0., scale.y, 0.));
            data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            tex[Direction::NegZ.to_idx()] as i32
        }
        BlockMeshShape::BottomSlab(height, tex) => {
            //TODO: ao strength should be reduced based on height
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);

            data.verts.push(origin + Vec3::new(0., 0., 0.));
            data.verts
                .push(origin + Vec3::new(0., height * scale.y, 0.));
            data.verts
                .push(origin + Vec3::new(scale.x, height * scale.y, 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));

            data.uvs.push(Vec2::new(1.0, *height));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, *height));

            tex[Direction::NegZ.to_idx()] as i32
        },
        BlockMeshShape::Cross([_, tex]) => {
            //ao not supported for this shape
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);

            //faces are angled at 45 degrees, so for the face to have unit side length
            //coordinates are (-sqrt(2)/4,-sqrt(2)/4) to (sqrt(2)/4,sqrt(2)/4)
            data.verts.push(origin + Vec3::new(0.5-SQRT_2_4,0.0,0.5-SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5-SQRT_2_4,1.0,0.5-SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5+SQRT_2_4,1.0,0.5+SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5+SQRT_2_4,0.0,0.5+SQRT_2_4)*scale);

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        },
        BlockMeshShape::Empty => {-1}
    };
    debug_assert_ne!(texture, -1);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);

    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
    data.norms.push(Vec3::new(0., 0., -1.));
}
pub fn mesh_pos_z(
    b: &BlockMeshShape,
    chunk: &impl Index<ChunkIdx, Output = BlockMesh>,
    coord: ChunkIdx,
    origin: Vec3,
    scale: Vec3,
    data: &mut MeshData,
) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
            add_ao(chunk, coord, false, false, true, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, false, true, true, data);

            data.verts.push(origin + Vec3::new(0., 0., scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts
                .push(origin + Vec3::new(scale.x, scale.y, scale.z));
            data.verts.push(origin + Vec3::new(0., scale.y, scale.z));

            data.uvs.push(Vec2::new(0.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));

            *tex as i32
        }
        BlockMeshShape::MultiTexture(tex) => {
            add_ao(chunk, coord, false, false, true, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, false, true, true, data);

            data.verts.push(origin + Vec3::new(0., 0., scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts
                .push(origin + Vec3::new(scale.x, scale.y, scale.z));
            data.verts.push(origin + Vec3::new(0., scale.y, scale.z));

            data.uvs.push(Vec2::new(0.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));

            tex[Direction::PosZ.to_idx()] as i32
        }
        BlockMeshShape::BottomSlab(height, tex) => {
            //TODO: ao strength should be reduced based on height
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);
            data.verts.push(origin + Vec3::new(0., 0., scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts
                .push(origin + Vec3::new(scale.x, height * scale.y, scale.z));
            data.verts
                .push(origin + Vec3::new(0., height * scale.y, scale.z));

            data.uvs.push(Vec2::new(0.0, 1.0 * height));
            data.uvs.push(Vec2::new(1.0, 1.0 * height));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));

            tex[Direction::PosZ.to_idx()] as i32
        },
        BlockMeshShape::Cross([_, tex]) => {
            //ao not supported for this shape
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);

            //faces are angled at 45 degrees, so for the face to have unit side length
            //coordinates are (-sqrt(2)/4,-sqrt(2)/4) to (sqrt(2)/4,sqrt(2)/4)
            data.verts.push(origin + Vec3::new(0.5+SQRT_2_4,0.0,0.5+SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5+SQRT_2_4,1.0,0.5+SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5-SQRT_2_4,1.0,0.5-SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5-SQRT_2_4,0.0,0.5-SQRT_2_4)*scale);

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        },
        BlockMeshShape::Empty => {-1}
    };
    debug_assert_ne!(texture, -1);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
    data.norms.push(Vec3::new(0., 0., 1.));
}

pub fn mesh_neg_x(
    b: &BlockMeshShape,
    chunk: &impl Index<ChunkIdx, Output = BlockMesh>,
    coord: ChunkIdx,
    origin: Vec3,
    scale: Vec3,
    data: &mut MeshData,
) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
            add_ao(chunk, coord, false, false, true, data);
            add_ao(chunk, coord, false, true, true, data);
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, false, false, false, data);

            data.verts.push(origin + Vec3::new(0., 0., scale.z));
            data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
            data.verts.push(origin + Vec3::new(0., scale.y, 0.));
            data.verts.push(origin + Vec3::new(0., 0., 0.));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        }
        BlockMeshShape::MultiTexture(tex) => {
            add_ao(chunk, coord, false, false, true, data);
            add_ao(chunk, coord, false, true, true, data);
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, false, false, false, data);

            data.verts.push(origin + Vec3::new(0., 0., scale.z));
            data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
            data.verts.push(origin + Vec3::new(0., scale.y, 0.));
            data.verts.push(origin + Vec3::new(0., 0., 0.));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            tex[Direction::NegX.to_idx()] as i32
        }
        BlockMeshShape::BottomSlab(height, tex) => {
            //TODO: ao strength should be reduced based on height
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);

            data.verts.push(origin + Vec3::new(0., 0., scale.z));
            data.verts
                .push(origin + Vec3::new(0., scale.y * height, scale.z));
            data.verts
                .push(origin + Vec3::new(0., scale.y * height, 0.));
            data.verts.push(origin + Vec3::new(0., 0., 0.));

            data.uvs.push(Vec2::new(1.0, 1.0 * height));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0 * height));

            tex[Direction::NegX.to_idx()] as i32
        }
        BlockMeshShape::Cross([tex, _]) => {
            //ao not supported for this shape
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);

            //faces are angled at 45 degrees, so for the face to have unit side length
            //coordinates are (-sqrt(2)/4,-sqrt(2)/4) to (sqrt(2)/4,sqrt(2)/4)
            data.verts.push(origin + Vec3::new(0.5-SQRT_2_4,0.0,0.5+SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5-SQRT_2_4,1.0,0.5+SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5+SQRT_2_4,1.0,0.5-SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5+SQRT_2_4,0.0,0.5-SQRT_2_4)*scale);
            
            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        },
        BlockMeshShape::Empty => {-1}
    };
    debug_assert_ne!(texture, -1);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
    data.norms.push(Vec3::new(-1., 0., 0.));
}

pub fn mesh_pos_x(
    b: &BlockMeshShape,
    chunk: &impl Index<ChunkIdx, Output = BlockMesh>,
    coord: ChunkIdx,
    origin: Vec3,
    scale: Vec3,
    data: &mut MeshData,
) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, true, false, data);

            data.verts
                .push(origin + Vec3::new(scale.x, scale.y, scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
            data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));

            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));

            *tex as i32
        }
        BlockMeshShape::MultiTexture(tex) => {
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, true, false, data);

            data.verts
                .push(origin + Vec3::new(scale.x, scale.y, scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
            data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));

            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));

            tex[Direction::PosX.to_idx()] as i32
        }
        BlockMeshShape::BottomSlab(height, tex) => {
            //TODO: ao strength should be reduced based on height
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);

            data.verts
                .push(origin + Vec3::new(scale.x, scale.y * height, scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
            data.verts
                .push(origin + Vec3::new(scale.x, scale.y * height, 0.));

            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));

            tex[Direction::PosX.to_idx()] as i32
        }
        BlockMeshShape::Cross([tex, _]) => {
            //ao not supported for this shape
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);

            //faces are angled at 45 degrees, so for the face to have unit side length
            //coordinates are (-sqrt(2)/4,-sqrt(2)/4) to (sqrt(2)/4,sqrt(2)/4)
            data.verts.push(origin + Vec3::new(0.5+SQRT_2_4,0.0,0.5-SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5+SQRT_2_4,1.0,0.5-SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5-SQRT_2_4,1.0,0.5+SQRT_2_4)*scale);
            data.verts.push(origin + Vec3::new(0.5-SQRT_2_4,0.0,0.5+SQRT_2_4)*scale);

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        },
        BlockMeshShape::Empty => {-1}
    };
    debug_assert_ne!(texture, -1);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
    data.norms.push(Vec3::new(1., 0., 0.));
}

pub fn mesh_pos_y(
    b: &BlockMeshShape,
    chunk: &impl Index<ChunkIdx, Output = BlockMesh>,
    coord: ChunkIdx,
    origin: Vec3,
    scale: Vec3,
    data: &mut MeshData,
) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, false, true, true, data);
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, true, true, false, data);

            data.verts.push(origin + Vec3::new(0., scale.y, 0.));
            data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
            data.verts
                .push(origin + Vec3::new(scale.x, scale.y, scale.z));
            data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        }
        BlockMeshShape::MultiTexture(tex) => {
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, false, true, true, data);
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, true, true, false, data);

            data.verts.push(origin + Vec3::new(0., scale.y, 0.));
            data.verts.push(origin + Vec3::new(0., scale.y, scale.z));
            data.verts
                .push(origin + Vec3::new(scale.x, scale.y, scale.z));
            data.verts.push(origin + Vec3::new(scale.x, scale.y, 0.));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            tex[Direction::PosY.to_idx()] as i32
        }
        BlockMeshShape::BottomSlab(height, tex) => {
            //TODO: ao strength should be reduced based on height
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);

            data.verts
                .push(origin + Vec3::new(0., scale.y * height, 0.));
            data.verts
                .push(origin + Vec3::new(0., scale.y * height, scale.z));
            data.verts
                .push(origin + Vec3::new(scale.x, scale.y * height, scale.z));
            data.verts
                .push(origin + Vec3::new(scale.x, scale.y * height, 0.));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            tex[Direction::PosY.to_idx()] as i32
        },
        BlockMeshShape::Cross(_) => {-1},
        BlockMeshShape::Empty => {-1}
    };
    debug_assert_ne!(texture, -1);
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));
    data.norms.push(Vec3::new(0., 1., 0.));

    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
}

pub fn mesh_neg_y(
    b: &BlockMeshShape,
    chunk: &impl Index<ChunkIdx, Output = BlockMesh>,
    coord: ChunkIdx,
    origin: Vec3,
    scale: Vec3,
    data: &mut MeshData,
) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);

            data.verts.push(origin + Vec3::new(0., 0., 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts.push(origin + Vec3::new(0., 0., scale.z));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(0.0, 1.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(1.0, 0.0));

            *tex as i32
        }
        BlockMeshShape::MultiTexture(tex) => {
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);

            data.verts.push(origin + Vec3::new(0., 0., 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts.push(origin + Vec3::new(0., 0., scale.z));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(0.0, 1.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(1.0, 0.0));

            tex[Direction::NegY.to_idx()] as i32
        }
        BlockMeshShape::BottomSlab(_, tex) => {
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);
            data.verts.push(origin + Vec3::new(0., 0., 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., 0.));
            data.verts.push(origin + Vec3::new(scale.x, 0., scale.z));
            data.verts.push(origin + Vec3::new(0., 0., scale.z));

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(0.0, 1.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(1.0, 0.0));

            tex[Direction::NegY.to_idx()] as i32
        }
        BlockMeshShape::Cross(_) => {-1},
        BlockMeshShape::Empty => {-1}
    };
    debug_assert_ne!(texture, -1);
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));
    data.norms.push(Vec3::new(0., -1., 0.));

    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
    data.layer_idx.push(texture);
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
    chunk: &impl Index<ChunkIdx, Output = BlockMesh>,
    //neighbors: &[Option<Chunk>; 6],
    coord: ChunkIdx,
    pos_x: bool,
    pos_y: bool,
    pos_z: bool,
    data: &mut MeshData,
) {
    fn should_add_ao(neighbor: &BlockMesh) -> bool {
        !matches!(neighbor.shape, BlockMeshShape::Empty) || !neighbor.use_transparent_shader
    }
    let side1_coord = IVec3::new(
        coord.x as i32 + if pos_x { 1 } else { -1 },
        coord.y as i32 + if pos_y { 1 } else { -1 },
        coord.z as i32,
    );
    let side2_coord = IVec3::new(
        coord.x as i32,
        coord.y as i32 + if pos_y { 1 } else { -1 },
        coord.z as i32 + if pos_z { 1 } else { -1 },
    );
    let corner_coord = IVec3::new(
        coord.x as i32 + if pos_x { 1 } else { -1 },
        coord.y as i32 + if pos_y { 1 } else { -1 },
        coord.z as i32 + if pos_z { 1 } else { -1 },
    );
    let mut side1 = false;
    let mut side2 = false;
    let mut corner = false;

    if side1_coord.x < CHUNK_SIZE_I32
        && side1_coord.x >= 0
        && side1_coord.y < CHUNK_SIZE_I32
        && side1_coord.y >= 0
    {
        side1 = should_add_ao(
            &chunk[ChunkIdx::new(
                side1_coord.x as u8,
                side1_coord.y as u8,
                side1_coord.z as u8
            )]
        );
    }
    if side2_coord.z < CHUNK_SIZE_I32
        && side2_coord.z >= 0
        && side2_coord.y < CHUNK_SIZE_I32
        && side2_coord.y >= 0
    {
        side2 = should_add_ao(
            &chunk[ChunkIdx::new(
                side2_coord.x as u8,
                side2_coord.y as u8,
                side2_coord.z as u8
            )]
        );
    }
    if corner_coord.x < CHUNK_SIZE_I32
        && corner_coord.x >= 0
        && corner_coord.y < CHUNK_SIZE_I32
        && corner_coord.y >= 0
        && corner_coord.z < CHUNK_SIZE_I32
        && corner_coord.z >= 0
    {
        corner = should_add_ao(
            &chunk[ChunkIdx::new(
                corner_coord.x as u8,
                corner_coord.y as u8,
                corner_coord.z as u8
            )]
        );
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
    LEVEL_FOR_NEIGHBORS[3 - (side1 as usize + side2 as usize + corner as usize)]
}
