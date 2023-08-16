use futures_lite::future;
use futures_timer::Delay;
use std::time::{Duration, Instant};

use crate::{
    mesher::NeedsMesh,
    physics::NeedsPhysics,
    util::{get_next_prng, SplineNoise},
    world::{
        chunk::*, BlockBuffer, BlockId, BlockName, BlockResources, Id, Level, LevelData,
        SavedBlockId, events::ChunkUpdatedEvent,
    }, worldgen::generator,
};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};

use super::{
    structures,
    DecorationResources, GenerationPhase, UsedShaperResources,
    ADD_TIME_BUDGET_MS, QUEUE_GEN_TIME_BUDGET_MS,
};

#[derive(Component)]
pub enum ChunkNeedsGenerated {
    Full,
    Lod(u8),
}

//task to generate the overall shape of the terrain
#[derive(Component)]
pub struct ShapingTask {
    pub task: Task<WaitingForDecoration>,
}

//wait for decoration constraints to be satisfied
#[derive(Component)]
pub struct WaitingForDecoration {
    pub chunk: ChunkCoord,
    pub heightmap: Heightmap<CHUNK_SIZE>,
}

//task to decorate (topsoil based on biome, flowers, grass, etc)
#[derive(Component)]
pub struct DecorationTask {
    pub task: Task<WaitingForStructures>,
}

//wait for structure constraints to be satisfied
#[derive(Component)]
pub struct WaitingForStructures {
    pub chunk: ChunkCoord,
    pub heightmap: Heightmap<CHUNK_SIZE>,
    pub biome_map: ColumnBiomes<CHUNK_SIZE>,
    pub structure_id: usize, //TODO: make this a type
}

//task to generate small structures (trees, buildings, etc)
#[derive(Component)]
pub struct StructureTask {
    pub task: Task<(ChunkCoord, BlockBuffer<BlockId>)>,
}

#[derive(Component)]
pub struct LODShapingTask {
    pub task: Task<GeneratingLODChunk>,
}

#[derive(Component)]
pub struct GeneratedChunk;

#[derive(Component)]
pub struct GeneratedLODChunk;

#[derive(Clone)]
pub struct Heightmap<const SIZE: usize>(pub [[f32; SIZE]; SIZE]);
#[derive(Clone)]
pub struct ColumnBiomes<const SIZE: usize>(pub [[Option<usize>; SIZE]; SIZE]);

pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub struct ShaperSettings<
    const NOISE: usize,
    const HEIGHTMAP: usize,
    const LANDMASS: usize,
    const SQUISH: usize,
> {
    //3d density "main" noise. value determines if a block is placed or not
    pub density_noise: SplineNoise<NOISE>,
    //2d low-frequency heightmap noise. creates whole landmasses and determines where oceans are
    pub landmass_noise: SplineNoise<LANDMASS>,
    //2d noise that squishes down variances in the heightmap
    pub squish_noise: SplineNoise<SQUISH>,
    //constant. value creates the upper control point for density required over the y axis
    pub upper_density: Vec2,
    //2d heightmap noise. value controls the x-value for the middle control point for density required over the y axis
    pub heightmap_noise: SplineNoise<HEIGHTMAP>,
    //constant. value controls the y-value for the middle control point for density required over the y axis
    pub mid_density: f32,
    //constant. value creates the lower control point for density required over the y axis
    pub lower_density: Vec2,
}

pub struct OreGenerator {
    pub ore_block: BlockId,
    pub can_replace: Vec<BlockId>,
    pub rarity: (u64, u64), //(numerator, denominator) proportion of chunks to generate a vein in
    pub vein_min: u32,
    pub vein_max: u32,
}

impl OreGenerator {
    pub fn get_ore_placement(&self, rng: u64) -> Option<ChunkIdx> {
        if rng % self.rarity.1 < self.rarity.0 {
            return None;
        }
        let x = get_next_prng(rng);
        let y = get_next_prng(x);
        let z = get_next_prng(y);
        Some(ChunkIdx::wrapped(x as u8, y as u8, z as u8))
    }
}

pub fn queue_generating<
    const NOISE: usize,
    const HEIGHTMAP: usize,
    const LANDMASS: usize,
    const SQUISH: usize,
>(
    query: Query<(Entity, &ChunkCoord, &ChunkNeedsGenerated)>,
    resources: Res<UsedShaperResources>,
    block_resources: Res<BlockResources>,
    level: Res<Level>,
    mut id: Local<SavedBlockId>,
    mut commands: Commands,
) {
    let _my_span = info_span!("queue_generating", name = "queue_generating").entered();
    if matches!(id.0, BlockId(Id::Empty)) {
        id.0 = block_resources.registry.get_id(&BlockName::core("stone"));
    }
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, coord, gen_request) in query.iter() {
        let gen_coord = *coord;
        let gen_noise = resources.0.clone();
        //must be async so that it's a future
        let mut ec = commands.entity(entity);
        ec.remove::<ChunkNeedsGenerated>();
        let id = id.0.clone();
        let level_data = level.0.clone();
        match gen_request {
            ChunkNeedsGenerated::Full => {
                ec.insert(ShapingTask {
                    task: pool.spawn(async move {
                        let mut chunk = GeneratingChunk::new(gen_coord, entity);
                        let heightmap = generator::shape_chunk(&mut chunk, gen_noise, id);
                        let ret = WaitingForDecoration {
                            chunk: chunk.position,
                            heightmap,
                        };
                        level_data.add_chunk(
                            chunk.position,
                            ChunkType::Generating(GenerationPhase::Shaped, chunk),
                        );
                        ret
                    }),
                });
            }
            ChunkNeedsGenerated::Lod(level) => {
                let gen_level = *level;
                ec.insert(LODShapingTask {
                    task: pool.spawn(async move {
                        let mut chunk = GeneratingLODChunk::new(gen_coord, entity);
                        chunk.level = gen_level;
                        generator::shape_chunk(&mut chunk, gen_noise, id);
                        chunk
                    }),
                });
            }
        };
        let duration = Instant::now().duration_since(now).as_millis();
        if duration > QUEUE_GEN_TIME_BUDGET_MS {
            break;
        }
    }
}

//ShapingTask -> WaitingForDecoration
pub fn poll_shaping_task(
    mut commands: Commands,
    mut shaping_query: Query<(Entity, &mut Transform, &mut ShapingTask)>,
) {
    let _my_span = info_span!("poll_shaping", name = "poll_shaping").entered();
    let now = Instant::now();
    for (entity, mut tf, mut task) in shaping_query.iter_mut() {
        if let Some(next) = future::block_on(future::poll_once(&mut task.task)) {
            tf.translation = next.chunk.to_vec3();
            commands.entity(entity).remove::<ShapingTask>().insert(next);
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

//WaitingForDecoration -> DecorationTask
pub fn poll_decoration_waiters(
    decor_resources: Res<DecorationResources>,
    level: Res<Level>,
    mut commands: Commands,
    mut watier_query: Query<(Entity, &WaitingForDecoration)>,
) {
    let _my_span = info_span!("poll_decor_waiters", name = "poll_decor_waiters").entered();
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, waiter) in watier_query.iter_mut() {
        if can_decorate(waiter.chunk, &level).is_some() {
            let settings = decor_resources.0.clone();
            let heightmap = waiter.heightmap.clone();
            let pos = waiter.chunk;
            let level = level.0.clone();
            commands
                .entity(entity)
                .remove::<WaitingForDecoration>()
                .insert(DecorationTask {
                    task: pool.spawn(async move {
                        let mut decor_requirements;
                        loop {
                            //keep polling until the requirements are satisfied
                            decor_requirements = can_decorate(pos, &level);
                            if decor_requirements.is_some() {
                                break;
                            }
                            //haven't had this happen, but if we hold this reference and then await it may cause a deadlock
                            drop(decor_requirements);
                            Delay::new(POLL_INTERVAL).await;
                        }
                        let (mut c, chunk_above) = decor_requirements.unwrap();
                        if let ChunkType::Generating(_, chunk) = c.value_mut() {
                            let biome_map = generator::gen_decoration(
                                chunk,
                                &chunk_above,
                                &heightmap,
                                &settings,
                            );
                            return WaitingForStructures {
                                chunk: pos,
                                heightmap,
                                biome_map,
                                structure_id: 0,
                            }
                        }
                        unreachable!()
                    }),
                });
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

fn can_decorate<'a>(
    chunk: ChunkCoord,
    level: &'a LevelData,
) -> Option<(
    dashmap::mapref::one::RefMut<'a, ChunkCoord, ChunkType, ahash::RandomState>,
    ChunkType,
)> {
    //can only hold one mutable reference into level without deadlocking, so we must clone the top_chunk 
    let top_chunk;
    match level.get_chunk(chunk + ChunkCoord::new(0, 1, 0)) {
        Some(top) => 
        match top.value() {
            ChunkType::Ungenerated(_) => return None,
            ChunkType::Generating(phase, _) => {
                if *phase >= GenerationPhase::Shaped {
                    top_chunk = top.value().clone();
                } else {
                    return None;
                }
            }
            ChunkType::Full(_) => top_chunk = top.value().clone(),
        },
        None => return None
    }
    if let Some(c) = level.get_chunk_mut(chunk) {
        if let ChunkType::Generating(phase, _) = c.value() {
            if *phase == GenerationPhase::Shaped {
                return Some((c, top_chunk));
            }
        }
    }
    None
}

//DecorationTask -> WaitingForStructures
pub fn poll_decoration_task(
    mut commands: Commands,
    mut decoration_query: Query<(Entity, &mut DecorationTask)>,
    level: Res<Level>
) {
    let _my_span = info_span!("poll_structure_waiters", name = "poll_structure_waiters").entered();
    let now = Instant::now();
    for (entity, mut task) in decoration_query.iter_mut() {
        if let Some(next) = future::block_on(future::poll_once(&mut task.task)) {
            level.update_chunk_phase(next.chunk, GenerationPhase::Decorated);
            commands
                .entity(entity)
                .remove::<DecorationTask>()
                .insert(next);
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

//WaitingForStructures -> StructureTask
pub fn poll_structure_waiters(
    decor_resources: Res<DecorationResources>,
    level: Res<Level>,
    mut commands: Commands,
    mut watier_query: Query<(Entity, &WaitingForStructures)>,
) {
    let _my_span = info_span!("poll_structure_waiters", name = "poll_structure_waiters").entered();
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, waiter) in watier_query.iter_mut() {
        if can_structure(waiter.chunk, &level).is_some() {
            let decor_settings = decor_resources.0.clone();
            let biomes = waiter.biome_map.clone();
            let level = level.0.clone();
            let pos = waiter.chunk;
            commands
                .entity(entity)
                .remove::<WaitingForStructures>()
                .insert(StructureTask {
                    task: pool
                        .spawn(async move { 
                            let mut structure_requirements;
                            loop {
                                //if the chu
                                structure_requirements = can_structure(pos, &level);
                                if structure_requirements.is_some() {
                                    break;
                                }
                                //haven't had this happen, but if we hold this reference and then await it may cause a deadlock
                                drop(structure_requirements);
                                Delay::new(POLL_INTERVAL).await;
                            }
                            let mut c = structure_requirements.unwrap();
                            if let ChunkType::Generating(_, ref mut chunk) = c.value_mut() {
                                let buf = structures::gen_structures(chunk, biomes, &decor_settings.biomes);
                                return (pos, buf)
                            }
                            unreachable!()
                     }),
                });
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

fn can_structure<'a>(
    chunk: ChunkCoord,
    level: &'a LevelData,
) -> Option<dashmap::mapref::one::RefMut<'a, ChunkCoord, ChunkType, ahash::RandomState>> {
    //this is very ugly but not sure how to make it better
    if let Some(mut c) = level.get_chunk_mut(chunk) {
        if let ChunkType::Generating(phase, _) = c.value_mut() {
            if *phase == GenerationPhase::Decorated {
                return Some(c);
            }
        }
    }
    None
}

//StructureTask -> finish
pub fn poll_structure_task(
    level: Res<Level>,
    resources: Res<BlockResources>,
    mut commands: Commands,
    mut decoration_query: Query<(Entity, &mut StructureTask)>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
) {
    let _my_span = info_span!("poll_structure_task", name = "poll_structure_task").entered();
    let now = Instant::now();
    for (entity, mut task) in decoration_query.iter_mut() {
        if let Some((pos, buf)) = future::block_on(future::poll_once(&mut task.task)) {
            level.add_buffer(
                buf.to_block_type(&resources.registry, &mut commands),
                &mut commands,
                &mut update_writer
            );
            level.promote_generating_to_full(pos, &resources.registry, &mut commands);
            commands
                .entity(entity)
                .remove::<StructureTask>()
                .insert(GeneratedChunk {})
                .insert(NeedsMesh {})
                .insert(NeedsPhysics {});
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
    level: Res<Level>,
    resources: Res<BlockResources>,
) {
    let _my_span = info_span!("poll_gen_lod_queue", name = "poll_gen_lod_queue").entered();
    let now = Instant::now();
    for (entity, mut tf, mut task) in query.iter_mut() {
        if let Some(mut data) = future::block_on(future::poll_once(&mut task.task)) {
            tf.translation = data.get_block_pos(ChunkIdx::new(0, 0, 0));
            commands.entity(entity).remove::<LODShapingTask>().insert((
                GeneratedLODChunk {},
                NeedsMesh {},
                LODLevel { level: data.level },
            ));
            level.add_lod_chunk(
                data.position,
                LODChunkType::Full(data.to_array_chunk(&resources.registry, &mut commands)),
            );
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

