use futures_lite::future;
use std::{time::Instant, sync::Arc};

use crate::{world::{chunk::*, Level, BlockType}, mesher::ChunkNeedsMesh, util::{Spline, SplineNoise}};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};

use super::{ADD_TIME_BUDGET_MS, QUEUE_GEN_TIME_BUDGET_MS};

#[derive(Component)]
pub struct ChunkNeedsGenerated {}

#[derive(Component)]
pub struct GenerationTask {
    pub task: Task<Chunk>,
}

#[derive(Component)]
pub struct GeneratedChunk {}

pub struct WorldGenSettings {
    //3d density "main" noise. value determines if a block is placed or not
    pub noise: SplineNoise,
    //constant. value creates the upper control point for density required over the y axis
    pub upper_density: Vec2,
    //2d heightmap noise. value controls the x-value for the middle control point for density required over the y axis
    pub heightmap_noise: SplineNoise,
    //constant. value controls the y-value for the middle control point for density required over the y axis
    pub mid_density: f32,
    //constant. value creates the lower control point for density required over the y axis
    pub base_density: Vec2
}

pub fn queue_generating(
    query: Query<(Entity, &ChunkCoord), With<ChunkNeedsGenerated>>,
    noise: Arc<WorldGenSettings>, //cannot use a resource since we pass it to other threads
    mut commands: Commands,
) {
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, coord) in query.iter() {
        let gen_coord = coord.clone();
        let gen_noise = noise.clone();
        let task = pool.spawn(async move { gen_chunk(gen_coord, entity,gen_noise) });
        commands
            .entity(entity)
            .remove::<ChunkNeedsGenerated>()
            .insert(GenerationTask { task });
        let duration = Instant::now().duration_since(now).as_millis();
        if duration > QUEUE_GEN_TIME_BUDGET_MS {
            break;
        }
    }
    // let duration = Instant::now().duration_since(now).as_millis();
    // if len > 0 {
    //     println!("queued mesh generation for {} chunks in {}ms", len, duration);
    // }
}

pub fn poll_gen_queue(
    mut commands: Commands,
    mut query: Query<(Entity, &mut GenerationTask)>,
    mut level: ResMut<Level>
) {
    let now = Instant::now();
    for (entity, mut task) in query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            commands
                .entity(entity)
                .remove::<GenerationTask>()
                .insert(GeneratedChunk {})
                .insert(ChunkNeedsMesh{});
            level.add_chunk(data.position, ChunkType::Full(data));
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

fn gen_chunk(coord: ChunkCoord, chunk_entity: Entity, settings: Arc<WorldGenSettings>) -> Chunk {
    let mut chunk = Chunk::new(coord, chunk_entity);
    let noise = &settings.noise;
    
    let chunk_pos = coord.to_vec3();
    for x in 0..CHUNK_SIZE_U8 {
        for y in 0..CHUNK_SIZE_U8 {
            let mut block_pos = Vec3::new(chunk_pos.x+x as f32,chunk_pos.y+y as f32,0.0);
            let density_map = Spline::new(&[settings.base_density, Vec2::new(settings.heightmap_noise.get_noise2d(block_pos.x, block_pos.y),settings.mid_density), settings.upper_density]);
            for z in 0..CHUNK_SIZE_U8 {
                block_pos.z = chunk_pos.z+z as f32;
                let density = noise.get_noise3d(block_pos.x,block_pos.y,block_pos.z);
                if density < density_map.map(block_pos.y) {
                    chunk[ChunkIdx::new(x,y,z)] = BlockType::Basic(0);
                }
            }
        }
    }
    chunk
}
