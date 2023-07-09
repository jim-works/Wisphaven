use futures_lite::future;
use std::{time::Instant, sync::Arc};

use crate::{world::{chunk::*, Level, BlockBuffer, BlockId, BlockName, BlockResources, SavedBlockId, Id}, mesher::NeedsMesh, util::{Spline, SplineNoise}, physics::NeedsPhysics};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};

use super::{ADD_TIME_BUDGET_MS, QUEUE_GEN_TIME_BUDGET_MS, structures::{self, StructureResources}};

#[derive(Component)]
pub enum ChunkNeedsGenerated {
    Full,
    Lod(u8)
}

//task to generate the overall shape of the terrain
#[derive(Component)]
pub struct ShapingTask {
    pub task: Task<GeneratingChunk>,
}

//task to generate small structures (trees, buildings, etc)
#[derive(Component)]
pub struct GenSmallStructureTask {
    pub task: Task<(GeneratingChunk, BlockBuffer<BlockId>)>,
}

#[derive(Component)]
pub struct LODShapingTask {
    pub task: Task<GeneratingLODChunk>,
}

#[derive(Component)]
pub struct GeneratedChunk;

#[derive(Component)]
pub struct GeneratedLODChunk;

pub struct ShaperSettings<const NOISE: usize, const HEIGHTMAP: usize> {
    //3d density "main" noise. value determines if a block is placed or not
    pub noise: SplineNoise<NOISE>,
    //constant. value creates the upper control point for density required over the y axis
    pub upper_density: Vec2,
    //2d heightmap noise. value controls the x-value for the middle control point for density required over the y axis
    pub heightmap_noise: SplineNoise<HEIGHTMAP>,
    //constant. value controls the y-value for the middle control point for density required over the y axis
    pub mid_density: f32,
    //constant. value creates the lower control point for density required over the y axis
    pub base_density: Vec2
}

pub fn queue_generating<const NOISE: usize, const HEIGHTMAP: usize>(
    query: Query<(Entity, &ChunkCoord, &ChunkNeedsGenerated)>,
    noise: Arc<ShaperSettings<NOISE,HEIGHTMAP>>, //cannot use a resource since we pass it to other threads
    block_resources: Res<BlockResources>,
    mut id: Local<SavedBlockId>,
    mut commands: Commands,
) {
    let _my_span = info_span!("queue_generating", name = "queue_generating").entered();
    if matches!(id.0, BlockId(Id::Empty)) {
        id.0 = block_resources.registry.get_id(&BlockName::core("grass"));
    }
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, coord, gen_request) in query.iter() {
        let gen_coord = *coord;
        let gen_noise = noise.clone();
        //must be async so that it's a future
        let mut ec = commands.entity(entity);
        ec.remove::<ChunkNeedsGenerated>();
        let id = id.0.clone();
        match gen_request {
            ChunkNeedsGenerated::Full => {
                ec.insert(ShapingTask { task: pool.spawn(async move { gen_chunk(gen_coord, entity,gen_noise, id) })});
            },
            ChunkNeedsGenerated::Lod(level) => {
                let gen_level = *level;
                ec.insert(LODShapingTask {task: pool.spawn(async move { gen_lod_chunk(gen_coord, gen_level, entity,gen_noise, id) })});
            },
        };
        let duration = Instant::now().duration_since(now).as_millis();
        if duration > QUEUE_GEN_TIME_BUDGET_MS {
            break;
        }
    }
}

pub fn poll_gen_queue(
    structure_resources: Res<StructureResources>,
    mut commands: Commands,
    mut shaping_query: Query<(Entity, &mut Transform, &mut ShapingTask)>,
    mut structure_query: Query<(Entity, &mut GenSmallStructureTask)>,
    resources: Res<BlockResources>,
    level: Res<Level>
) {
    let _my_span = info_span!("poll_gen_queue", name = "poll_gen_queue").entered();
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, mut tf, mut task) in shaping_query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            tf.translation = data.position.to_vec3();
            let settings = structure_resources.settings.clone();
            commands
                .entity(entity)
                .remove::<ShapingTask>()
                .insert(GenSmallStructureTask {
                    task: pool.spawn(async move { structures::gen_small_structures(data, settings) })
                });
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
    for (entity, mut task) in structure_query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            level.add_buffer(data.1.to_block_type(resources.registry.as_ref(), &mut commands), &mut commands);
            level.add_chunk(data.0.position, ChunkType::Full(data.0.to_array_chunk(resources.registry.as_ref(), &mut commands)));
            commands
                .entity(entity)
                .remove::<GenSmallStructureTask>()
                .insert(GeneratedChunk {})
                .insert(NeedsMesh{})
                .insert(NeedsPhysics{});
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

pub fn poll_gen_lod_queue(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut LODShapingTask)>,
    mut level: ResMut<Level>,
    resources: Res<BlockResources>
) {
    let _my_span = info_span!("poll_gen_lod_queue", name = "poll_gen_lod_queue").entered();
    let now = Instant::now();
    for (entity, mut tf, mut task) in query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            tf.translation = data.get_block_pos(ChunkIdx::new(0,0,0));
            commands
                .entity(entity)
                .remove::<LODShapingTask>()
                .insert((GeneratedLODChunk {}, NeedsMesh{}, LODLevel{level: data.level}));
            level.add_lod_chunk(data.position, LODChunkType::Full(data.to_array_chunk(resources.registry.as_ref(), &mut commands)));
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

fn gen_chunk<const NOISE: usize, const HEIGHTMAP: usize>(coord: ChunkCoord, chunk_entity: Entity, settings: Arc<ShaperSettings<NOISE,HEIGHTMAP>>, block_id: BlockId) -> GeneratingChunk {
    let _my_span = info_span!("gen_chunk", name = "gen_chunk").entered();
    let mut chunk = GeneratingChunk::new(coord, chunk_entity);
    let noise = &settings.noise;
    
    let chunk_pos = coord.to_vec3();
    for x in 0..CHUNK_SIZE_U8 {
        for y in 0..CHUNK_SIZE_U8 {
            let mut block_pos = Vec3::new(chunk_pos.x+x as f32,chunk_pos.y+y as f32,0.0);
            let density_map = Spline::new([settings.base_density, Vec2::new(settings.heightmap_noise.get_noise2d(block_pos.x, block_pos.y),settings.mid_density), settings.upper_density]);
            for z in 0..CHUNK_SIZE_U8 {
                block_pos.z = chunk_pos.z+z as f32;
                let density = noise.get_noise3d(block_pos.x,block_pos.y,block_pos.z);
                 if density < density_map.map(block_pos.y) {
                    chunk[ChunkIdx::new(x,y,z)] = block_id;
                }
            }
        }
    }
    chunk
}

fn gen_lod_chunk<const NOISE: usize, const HEIGHTMAP: usize>(coord: ChunkCoord, level: u8, chunk_entity: Entity, settings: Arc<ShaperSettings<NOISE,HEIGHTMAP>>, block_id: BlockId) -> GeneratingLODChunk {
    let _my_span = info_span!("gen_lod_chunk", name = "gen_lod_chunk").entered();
    let mut chunk = GeneratingLODChunk::new(coord, chunk_entity);
    chunk.level = level;
    let noise = &settings.noise;
    
    for x in 0..CHUNK_SIZE_U8 {
        for y in 0..CHUNK_SIZE_U8 {
            let mut block_pos = chunk.get_block_pos(ChunkIdx::new(x,y,0));
            let density_map = Spline::new([settings.base_density, Vec2::new(settings.heightmap_noise.get_noise2d(block_pos.x, block_pos.y),settings.mid_density), settings.upper_density]);
            for z in 0..CHUNK_SIZE_U8 {
                block_pos = chunk.get_block_pos(ChunkIdx::new(x,y,z));
                let density = noise.get_noise3d(block_pos.x,block_pos.y,block_pos.z);
                if density < density_map.map(block_pos.y) {
                    chunk[ChunkIdx::new(x,y,z)] = block_id;
                }
            }
        }
    }
    chunk
}
