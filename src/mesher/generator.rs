use bevy::pbr::{ExtendedMaterial, NotShadowCaster};
use futures_lite::future;
use std::ops::Index;
use std::time::Instant;

use crate::util::{Corner, Edge};
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

use super::extended_materials::TextureArrayExtension;
use super::is_chunk_ready_for_meshing;
use super::materials::ATTRIBUTE_AO;
use super::{materials::ATTRIBUTE_TEXLAYER, ChunkMaterial, SPAWN_MESH_TIME_BUDGET_COUNT};

#[derive(Component, Default)]
pub struct NeedsMesh {
    pub order: Option<usize>,
}

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

const SQRT_2_4: f32 = 0.35355339; //sqrt(2)/4

pub fn queue_meshing(
    query: Query<(Entity, &ChunkCoord, &NeedsMesh), (With<GeneratedChunk>, Without<DontMeshChunk>)>,
    currently_meshing: Query<(), With<MeshTask>>,
    level: Res<Level>,
    mesh_query: Query<&BlockMesh>,
    commands: ParallelCommands,
) {
    let _my_span = info_span!("queue_meshing", name = "queue_meshing").entered();
    let pool = AsyncComputeTaskPool::get();
    //todo - set based on core count
    let max_mesh_count = 64; //don't launch new mesh tasks if there's more than this
    if currently_meshing.iter().len() > max_mesh_count {
        return;
    }
    //want to avoid sorting computation, so we only mesh the chunks with order in [max_order-order_tolerance, max_order]
    //order is lowest first
    let order_tolerance: usize = 4;
    let max_order_opt = query
        .iter()
        .map(|(_, _, m)| m.order)
        .min_by_key(|f| f.unwrap_or(usize::MAX - order_tolerance)) //max_order + order_tolerance within bounds
        .flatten();

    if let Some(max_order) = max_order_opt {
        query.par_iter().for_each(|(entity, coord, needs_mesh)| {
            if needs_mesh
                .order
                .map_or(false, |order| max_order + order_tolerance < order)
            {
                //too late in the order, don't mesh yet
                return;
            }
            if let Some(ctype) = level.get_chunk(*coord) {
                if let ChunkType::Full(chunk) = ctype.value() {
                    if !is_chunk_ready_for_meshing(*coord, &level) {
                        //chunk not ready
                        return;
                    }
                    //we have to check neighbor counts again because a chunk could be removed since the last loop, and we don't clone anything before
                    let mut ready_neighbors = 0;
                    let mut face_neighbors = [None, None, None, None, None, None];
                    let mut edge_neighbors = [
                        None, None, None, None, None, None, None, None, None, None, None, None,
                    ];
                    let mut corner_neighbors = [None, None, None, None, None, None, None, None];
                    for dir in Direction::iter() {
                        if let Some(ctype) = level.get_chunk(coord.offset(dir)) {
                            if let ChunkType::Full(neighbor) = ctype.value() {
                                ready_neighbors += 1;
                                face_neighbors[dir.to_idx()] =
                                    Some(neighbor.with_storage(Box::new(
                                        neighbor.blocks.get_components::<BlockMesh>(&mesh_query),
                                    )));
                            }
                        }
                    }
                    for dir in Corner::iter() {
                        if let Some(ctype) = level.get_chunk(*coord + dir.into()) {
                            if let ChunkType::Full(neighbor) = ctype.value() {
                                ready_neighbors += 1;
                                corner_neighbors[dir as usize] =
                                    Some(neighbor.blocks.get_component::<BlockMesh>(
                                        Into::<ChunkIdx>::into(dir.opposite()).into(),
                                        &mesh_query,
                                    ));
                            }
                        }
                    }
                    for dir in Edge::iter() {
                        if let Some(ctype) = level.get_chunk(*coord + dir.into()) {
                            if let ChunkType::Full(neighbor) = ctype.value() {
                                ready_neighbors += 1;
                                let origin = dir.opposite().origin();
                                let direction = dir.opposite().direction();
                                edge_neighbors[dir as usize] = Some(core::array::from_fn(|i| {
                                    neighbor.blocks.get_component::<BlockMesh>(
                                        ChunkIdx::new(
                                            (origin.x as i32 + i as i32 * direction.x) as u8,
                                            (origin.y as i32 + i as i32 * direction.y) as u8,
                                            (origin.z as i32 + i as i32 * direction.z) as u8,
                                        )
                                        .into(),
                                        &mesh_query,
                                    )
                                }));
                            }
                        }
                    }

                    if ready_neighbors != 26 {
                        //don't mesh if all neighbors aren't ready yet
                        return;
                    }
                    let meshing = chunk.with_storage(Box::new(chunk.blocks.create_fat_palette(
                        &mesh_query,
                        face_neighbors,
                        edge_neighbors,
                        corner_neighbors,
                    )));
                    let task = pool.spawn(async move {
                        let mut data = ChunkMesh::new(1.0);
                        mesh_chunk(&meshing, &mut data);
                        data
                    });
                    commands.command_scope(|mut commands| {
                        commands
                            .entity(entity)
                            .remove::<NeedsMesh>()
                            .insert(MeshTask { task });
                    });
                }
            }
        });
    }
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
                spawn_mesh::<()>(
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

pub fn create_mesh(data: MeshData, meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.verts);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.norms);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs);
    mesh.insert_attribute(ATTRIBUTE_TEXLAYER, data.layer_idx);
    mesh.insert_attribute(ATTRIBUTE_AO, data.ao_level);

    mesh.set_indices(Some(mesh::Indices::U32(data.tris)));
    meshes.add(mesh)
}

pub fn spawn_mesh<T: Bundle>(
    data: MeshData,
    material: Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    components: Option<T>,
    entity: Entity,
) {
    commands.entity(entity).with_children(|children| {
        let mut ec = children.spawn((
            MaterialMeshBundle::<ExtendedMaterial<StandardMaterial, TextureArrayExtension>> {
                mesh: create_mesh(data, meshes),
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

fn mesh_chunk<T: ChunkStorage<BlockMesh>>(fat_chunk: &Chunk<T, BlockMesh>, data: &mut ChunkMesh) {
    let _my_span = info_span!("mesh_chunk", name = "mesh_chunk").entered();
    for x in 0..CHUNK_SIZE_I8 {
        for y in 0..CHUNK_SIZE_I8 {
            for z in 0..CHUNK_SIZE_I8 {
                let coord = FatChunkIdx::new(x, y, z);
                mesh_block(
                    fat_chunk,
                    &fat_chunk[Into::<usize>::into(coord)],
                    coord,
                    Into::<ChunkIdx>::into(coord).to_vec3() * data.scale,
                    data,
                )
            }
        }
    }
}
pub fn should_mesh_face(block: &BlockMesh, block_face: Direction, neighbor: &BlockMesh) -> bool {
    has_face(block, block_face) && face_showing(block, block_face, neighbor)
}

pub fn has_face(block: &BlockMesh, block_face: Direction) -> bool {
    match block.shape {
        BlockMeshShape::Empty => false,
        BlockMeshShape::Uniform(_)
        | BlockMeshShape::MultiTexture(_)
        | BlockMeshShape::BottomSlab(_, _) => true,
        BlockMeshShape::Cross(_) => !matches!(block_face, Direction::PosY | Direction::NegY),
    }
}

pub fn face_showing(block: &BlockMesh, block_face: Direction, neighbor: &BlockMesh) -> bool {
    if !block.use_transparent_shader && neighbor.use_transparent_shader {
        return true;
    }
    match block.shape {
        BlockMeshShape::Uniform(_) | BlockMeshShape::MultiTexture(_) => {
            block != neighbor && neighbor.shape.is_transparent(block_face.opposite())
        }
        BlockMeshShape::BottomSlab(_, _) => {
            block_face == Direction::PosY
                || block != neighbor && neighbor.shape.is_transparent(block_face.opposite())
        }
        BlockMeshShape::Cross(_) => true,
        BlockMeshShape::Empty => false,
    }
}

//meshes a full block and returns the handle
pub fn mesh_single_block(b: &BlockMesh, meshes: &mut ResMut<Assets<Mesh>>) -> Option<Handle<Mesh>> {
    if matches!(b.shape, BlockMeshShape::Empty) {
        return None;
    }
    let mut mesh = MeshData::new();
    if has_face(b, Direction::PosZ) {
        mesh_pos_z(&b.shape, Vec3::ZERO, Vec3::ONE, &mut mesh);
        mesh.ao_level.extend([1.0; 4]);
    }
    if has_face(b, Direction::NegZ) {
        mesh_neg_z(&b.shape, Vec3::ZERO, Vec3::ONE, &mut mesh);
        mesh.ao_level.extend([1.0; 4]);
    }
    if has_face(b, Direction::PosX) {
        mesh_pos_x(&b.shape, Vec3::ZERO, Vec3::ONE, &mut mesh);
        mesh.ao_level.extend([1.0; 4]);
    }
    if has_face(b, Direction::NegX) {
        mesh_neg_x(&b.shape, Vec3::ZERO, Vec3::ONE, &mut mesh);
        mesh.ao_level.extend([1.0; 4]);
    }
    if has_face(b, Direction::PosY) {
        mesh_pos_y(&b.shape, Vec3::ZERO, Vec3::ONE, &mut mesh);
        mesh.ao_level.extend([1.0; 4]);
    }
    if has_face(b, Direction::NegY) {
        mesh_neg_y(&b.shape, Vec3::ZERO, Vec3::ONE, &mut mesh);
        mesh.ao_level.extend([1.0; 4]);
    }
    Some(create_mesh(mesh, meshes))
}
fn mesh_block<T: ChunkStorage<BlockMesh>>(
    fat_chunk: &Chunk<T, BlockMesh>,
    b: &BlockMesh,
    coord: FatChunkIdx,
    origin: Vec3,
    data: &mut ChunkMesh,
) {
    if matches!(b.shape, BlockMeshShape::Empty) {
        return;
    }
    let selected_data = if b.use_transparent_shader {
        &mut data.transparent
    } else {
        &mut data.opaque
    };
    if should_mesh_face(
        b,
        Direction::PosZ,
        &fat_chunk[Into::<usize>::into(FatChunkIdx::new(coord.x, coord.y, coord.z + 1))],
    ) {
        mesh_pos_z(
            &b.shape,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            selected_data,
        );
        add_ao_pos_z(&b.shape, fat_chunk, coord, selected_data)
    }
    //negative z face
    if should_mesh_face(
        b,
        Direction::NegZ,
        &fat_chunk[Into::<usize>::into(FatChunkIdx::new(coord.x, coord.y, coord.z - 1))],
    ) {
        mesh_neg_z(
            &b.shape,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            selected_data,
        );
        add_ao_neg_z(&b.shape, fat_chunk, coord, selected_data)
    }
    //positive y face
    if should_mesh_face(
        b,
        Direction::PosY,
        &fat_chunk[Into::<usize>::into(FatChunkIdx::new(coord.x, coord.y + 1, coord.z))],
    ) {
        mesh_pos_y(
            &b.shape,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            selected_data,
        );
        add_ao_pos_y(&b.shape, fat_chunk, coord, selected_data)
    }
    //negative y face
    if should_mesh_face(
        b,
        Direction::NegY,
        &fat_chunk[Into::<usize>::into(FatChunkIdx::new(coord.x, coord.y - 1, coord.z))],
    ) {
        mesh_neg_y(
            &b.shape,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            selected_data,
        );
        add_ao_neg_y(&b.shape, fat_chunk, coord, selected_data)
    }
    //positive x face
    if should_mesh_face(
        b,
        Direction::PosX,
        &fat_chunk[Into::<usize>::into(FatChunkIdx::new(coord.x + 1, coord.y, coord.z))],
    ) {
        mesh_pos_x(
            &b.shape,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            selected_data,
        );
        add_ao_pos_x(&b.shape, fat_chunk, coord, selected_data)
    }
    //negative x face
    if should_mesh_face(
        b,
        Direction::NegX,
        &fat_chunk[Into::<usize>::into(FatChunkIdx::new(coord.x - 1, coord.y, coord.z))],
    ) {
        mesh_neg_x(
            &b.shape,
            origin,
            Vec3::new(data.scale, data.scale, data.scale),
            selected_data,
        );
        add_ao_neg_x(&b.shape, fat_chunk, coord, selected_data)
    }
}

pub fn add_ao_neg_z(
    b: &BlockMeshShape,
    chunk: &impl Index<usize, Output = BlockMesh>,
    coord: FatChunkIdx,
    data: &mut MeshData,
) {
    match b {
        BlockMeshShape::Empty => {}
        BlockMeshShape::Cross(_) => {
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
        }
        _ => {
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, true, true, false, data);
            add_ao(chunk, coord, true, false, false, data);
        }
    }
}

//TODO: Set uv scale for repeating textures
//note: must add ao information separately
pub fn mesh_neg_z(b: &BlockMeshShape, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
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
        }
        BlockMeshShape::Cross([_, tex]) => {
            //faces are angled at 45 degrees, so for the face to have unit side length
            //coordinates are (-sqrt(2)/4,-sqrt(2)/4) to (sqrt(2)/4,sqrt(2)/4)
            data.verts
                .push(origin + Vec3::new(0.5 - SQRT_2_4, 0.0, 0.5 - SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 - SQRT_2_4, 1.0, 0.5 - SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 + SQRT_2_4, 1.0, 0.5 + SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 + SQRT_2_4, 0.0, 0.5 + SQRT_2_4) * scale);

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        }
        BlockMeshShape::Empty => -1,
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

pub fn add_ao_pos_z(
    b: &BlockMeshShape,
    chunk: &impl Index<usize, Output = BlockMesh>,
    coord: FatChunkIdx,
    data: &mut MeshData,
) {
    match b {
        BlockMeshShape::Empty => {}
        BlockMeshShape::Cross(_) => {
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
        }
        _ => {
            add_ao(chunk, coord, false, false, true, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, false, true, true, data);
        }
    }
}

//note: must add ao information separately
pub fn mesh_pos_z(b: &BlockMeshShape, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
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
        }
        BlockMeshShape::Cross([_, tex]) => {
            //faces are angled at 45 degrees, so for the face to have unit side length
            //coordinates are (-sqrt(2)/4,-sqrt(2)/4) to (sqrt(2)/4,sqrt(2)/4)
            data.verts
                .push(origin + Vec3::new(0.5 + SQRT_2_4, 0.0, 0.5 + SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 + SQRT_2_4, 1.0, 0.5 + SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 - SQRT_2_4, 1.0, 0.5 - SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 - SQRT_2_4, 0.0, 0.5 - SQRT_2_4) * scale);

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        }
        BlockMeshShape::Empty => -1,
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

pub fn add_ao_neg_x(
    b: &BlockMeshShape,
    chunk: &impl Index<usize, Output = BlockMesh>,
    coord: FatChunkIdx,
    data: &mut MeshData,
) {
    match b {
        BlockMeshShape::Empty => {}
        BlockMeshShape::Cross(_) => {
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
        }
        _ => {
            add_ao(chunk, coord, false, false, true, data);
            add_ao(chunk, coord, false, true, true, data);
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, false, false, false, data);
        }
    }
}

//note: must add ao information separately
pub fn mesh_neg_x(b: &BlockMeshShape, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
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
            //faces are angled at 45 degrees, so for the face to have unit side length
            //coordinates are (-sqrt(2)/4,-sqrt(2)/4) to (sqrt(2)/4,sqrt(2)/4)
            data.verts
                .push(origin + Vec3::new(0.5 - SQRT_2_4, 0.0, 0.5 + SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 - SQRT_2_4, 1.0, 0.5 + SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 + SQRT_2_4, 1.0, 0.5 - SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 + SQRT_2_4, 0.0, 0.5 - SQRT_2_4) * scale);

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        }
        BlockMeshShape::Empty => -1,
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

pub fn add_ao_pos_x(
    b: &BlockMeshShape,
    chunk: &impl Index<usize, Output = BlockMesh>,
    coord: FatChunkIdx,
    data: &mut MeshData,
) {
    match b {
        BlockMeshShape::Empty => {}
        BlockMeshShape::Cross(_) => {
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
            data.ao_level.push(1.0);
        }
        _ => {
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, true, false, data);
        }
    }
}

//note: must add ao information separately
pub fn mesh_pos_x(b: &BlockMeshShape, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
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
            //faces are angled at 45 degrees, so for the face to have unit side length
            //coordinates are (-sqrt(2)/4,-sqrt(2)/4) to (sqrt(2)/4,sqrt(2)/4)
            data.verts
                .push(origin + Vec3::new(0.5 + SQRT_2_4, 0.0, 0.5 - SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 + SQRT_2_4, 1.0, 0.5 - SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 - SQRT_2_4, 1.0, 0.5 + SQRT_2_4) * scale);
            data.verts
                .push(origin + Vec3::new(0.5 - SQRT_2_4, 0.0, 0.5 + SQRT_2_4) * scale);

            data.uvs.push(Vec2::new(1.0, 1.0));
            data.uvs.push(Vec2::new(1.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 0.0));
            data.uvs.push(Vec2::new(0.0, 1.0));

            *tex as i32
        }
        BlockMeshShape::Empty => -1,
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

pub fn add_ao_pos_y(
    b: &BlockMeshShape,
    chunk: &impl Index<usize, Output = BlockMesh>,
    coord: FatChunkIdx,
    data: &mut MeshData,
) {
    match b {
        BlockMeshShape::Empty | BlockMeshShape::Cross(_) => {}
        _ => {
            add_ao(chunk, coord, false, true, false, data);
            add_ao(chunk, coord, false, true, true, data);
            add_ao(chunk, coord, true, true, true, data);
            add_ao(chunk, coord, true, true, false, data);
        }
    }
}

//note: must add ao information separately
pub fn mesh_pos_y(b: &BlockMeshShape, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
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
        }
        BlockMeshShape::Cross(_) => -1,
        BlockMeshShape::Empty => -1,
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

pub fn add_ao_neg_y(
    b: &BlockMeshShape,
    chunk: &impl Index<usize, Output = BlockMesh>,
    coord: FatChunkIdx,
    data: &mut MeshData,
) {
    match b {
        BlockMeshShape::Empty | BlockMeshShape::Cross(_) => {}
        _ => {
            add_ao(chunk, coord, false, false, false, data);
            add_ao(chunk, coord, true, false, false, data);
            add_ao(chunk, coord, true, false, true, data);
            add_ao(chunk, coord, false, false, true, data);
        }
    }
}
//note: must add ao information separately
pub fn mesh_neg_y(b: &BlockMeshShape, origin: Vec3, scale: Vec3, data: &mut MeshData) {
    add_tris(&mut data.tris, data.verts.len() as u32);
    let texture = match b {
        BlockMeshShape::Uniform(tex) => {
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
        BlockMeshShape::Cross(_) => -1,
        BlockMeshShape::Empty => -1,
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
    chunk: &impl Index<usize, Output = BlockMesh>,
    coord: FatChunkIdx,
    pos_x: bool,
    pos_y: bool,
    pos_z: bool,
    data: &mut MeshData,
) {
    fn should_add_ao(neighbor: &BlockMesh) -> bool {
        !neighbor.use_transparent_shader && !matches!(neighbor.shape, BlockMeshShape::Empty)
    }
    let side1_coord = FatChunkIdx::new(
        coord.x + if pos_x { 1 } else { -1 },
        coord.y + if pos_y { 1 } else { -1 },
        coord.z,
    );
    let side2_coord = FatChunkIdx::new(
        coord.x,
        coord.y + if pos_y { 1 } else { -1 },
        coord.z + if pos_z { 1 } else { -1 },
    );
    let corner_coord = FatChunkIdx::new(
        coord.x + if pos_x { 1 } else { -1 },
        coord.y + if pos_y { 1 } else { -1 },
        coord.z + if pos_z { 1 } else { -1 },
    );
    let side1 = should_add_ao(&chunk[side1_coord.into()]);
    let side2 = should_add_ao(&chunk[side2_coord.into()]);
    let corner = should_add_ao(&chunk[corner_coord.into()]);
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
