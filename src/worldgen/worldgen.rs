use bracket_noise::prelude::FastNoise;
use futures_lite::future;
use std::{time::Instant, sync::Arc};

use crate::{world::{chunk::*, Level}, mesher::ChunkNeedsMesh};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};

use super::{ADD_TIME_BUDGET_COUNT, QUEUE_GEN_TIME_BUDGET_MS};

#[derive(Component)]
pub struct ChunkNeedsGenerated {}

#[derive(Component)]
pub struct GenerationTask {
    pub task: Task<Chunk>,
}

#[derive(Component)]
pub struct GeneratedChunk {}

pub struct WorldGenSettings {
    noise: FastNoise,

}

pub fn queue_generating(
    query: Query<(Entity, &ChunkCoord), With<ChunkNeedsGenerated>>,
    noise: Arc<FastNoise>,
    mut commands: Commands,
) {
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, coord) in query.iter() {
        let gen_coord = coord.clone();
        let gen_noise = noise.clone();
        let task = pool.spawn(async move { gen_chunk(gen_coord, gen_noise) });
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
            level.add_chunk(data.position, data);
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_COUNT {
                break;
            }
        }
    }
}

fn gen_chunk(coord: ChunkCoord, noise: Arc<FastNoise>) -> Chunk {
    let mut chunk = Chunk::new(coord);
    let chunk_pos = coord.to_vec3();
    for x in 0..CHUNK_SIZE_U8 {
        for y in 0..CHUNK_SIZE_U8 {
            for z in 0..CHUNK_SIZE_U8 {
                let density = noise.get_noise3d(chunk_pos.x+x as f32,chunk_pos.y+y as f32,chunk_pos.z+z as f32);
                if density > 0. {
                    chunk[ChunkIdx::new(x,y,z)] = BlockType::Basic(0);
                }
            }
        }
    }
    chunk
}
