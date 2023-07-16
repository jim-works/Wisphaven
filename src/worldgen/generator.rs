use futures_lite::future;
use std::{sync::Arc, time::Instant};

use crate::{
    mesher::NeedsMesh,
    physics::NeedsPhysics,
    util::{get_next_prng, trilerp, ClampedSpline, SplineNoise},
    world::{chunk::*, BlockBuffer, BlockId, BlockName, BlockResources, Id, Level, SavedBlockId},
};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};

use super::{
    structures::{self, StructureResources},
    DecorationResources, UsedShaperResources, ADD_TIME_BUDGET_MS, QUEUE_GEN_TIME_BUDGET_MS, DecorationSettings,
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
    pub chunk: GeneratingChunk,
    pub heightmap: Heightmap<CHUNK_SIZE>
}

//task to decorate (topsoil based on biome, flowers, grass, etc)
#[derive(Component)]
pub struct DecorationTask {
    pub task: Task<WaitingForStructures>,
}

//wait for structure constraints to be satisfied
#[derive(Component)]
pub struct WaitingForStructures {
    pub chunk: GeneratingChunk,
    pub heightmap: Heightmap<CHUNK_SIZE>,
    pub biome_map: ColumnBiomes<CHUNK_SIZE>,
    pub structure_id: usize, //TODO: make this a type
}

//task to generate small structures (trees, buildings, etc)
#[derive(Component)]
pub struct StructureTask {
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

#[derive(Clone)]
pub struct Heightmap<const SIZE: usize>([[f32; SIZE]; SIZE]);
#[derive(Clone)]
pub struct ColumnBiomes<const SIZE: usize>([[Option<usize>; SIZE]; SIZE]);

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
        match gen_request {
            ChunkNeedsGenerated::Full => {
                ec.insert(ShapingTask {
                    task: pool.spawn(async move {
                        let mut chunk = GeneratingChunk::new(gen_coord, entity);
                        let heightmap = shape_chunk(&mut chunk, gen_noise, id);
                        WaitingForDecoration {chunk, heightmap}
                    }),
                });
            }
            ChunkNeedsGenerated::Lod(level) => {
                let gen_level = *level;
                ec.insert(LODShapingTask {
                    task: pool.spawn(async move {
                        let mut chunk = GeneratingLODChunk::new(gen_coord, entity);
                        chunk.level = gen_level;
                        shape_chunk(&mut chunk, gen_noise, id);
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
            tf.translation = next.chunk.position.to_vec3();
            commands
                .entity(entity)
                .remove::<ShapingTask>()
                .insert(next);
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
        if can_decorate(&waiter.chunk, level.as_ref()) {
            let settings = decor_resources.0.clone();
            let chunk = waiter.chunk.clone();
            let heightmap = waiter.heightmap.clone();
            commands
                .entity(entity)
                .remove::<WaitingForDecoration>()
                .insert(DecorationTask {
                    task: pool.spawn(async move { 
                        let (chunk, biome_map) = gen_decoration(chunk, &heightmap, settings.as_ref());
                        WaitingForStructures {
                            chunk,
                            heightmap,
                            biome_map,
                            structure_id: 0,
                        }
                     }),
                });
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

fn can_decorate(
    _chunk: &GeneratingChunk,
    _level: &Level
) -> bool {
    true
}

//DecorationTask -> WaitingForStructures
pub fn poll_decoration_task(
    mut commands: Commands,
    mut decoration_query: Query<(Entity, &mut DecorationTask)>,
) {
    let _my_span = info_span!("poll_structure_waiters", name = "poll_structure_waiters").entered();
    let now = Instant::now();
    for (entity, mut task) in decoration_query.iter_mut() {
        if let Some(next) = future::block_on(future::poll_once(&mut task.task)) {
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
    structure_resources: Res<StructureResources>,
    level: Res<Level>,
    mut commands: Commands,
    mut watier_query: Query<(Entity, &WaitingForStructures)>,
) {
    let _my_span = info_span!("poll_structure_waiters", name = "poll_structure_waiters").entered();
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, waiter) in watier_query.iter_mut() {
        if can_structure(&waiter.chunk, level.as_ref()) {
            let settings = structure_resources.settings.clone();
            let chunk = waiter.chunk.clone();
            commands
                .entity(entity)
                .remove::<WaitingForStructures>()
                .insert(StructureTask {
                    task: pool.spawn(async move { 
                        structures::gen_small_structures(chunk, settings)
                     }),
                });
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

fn can_structure(
    _chunk: &GeneratingChunk,
    _level: &Level
) -> bool {
    true
}

//StructureTask -> finish
pub fn poll_structure_task(
    level: Res<Level>,
    resources: Res<BlockResources>,
    mut commands: Commands,
    mut decoration_query: Query<(Entity, &mut StructureTask)>,
) {
    let _my_span = info_span!("poll_structure_task", name = "poll_structure_task").entered();
    let now = Instant::now();
    for (entity, mut task) in decoration_query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            level.add_buffer(
                data.1
                    .to_block_type(resources.registry.as_ref(), &mut commands),
                &mut commands,
            );
            level.add_chunk(
                data.0.position,
                ChunkType::Full(
                    data.0
                        .to_array_chunk(resources.registry.as_ref(), &mut commands),
                ),
            );
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
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            tf.translation = data.get_block_pos(ChunkIdx::new(0, 0, 0));
            commands.entity(entity).remove::<LODShapingTask>().insert((
                GeneratedLODChunk {},
                NeedsMesh {},
                LODLevel { level: data.level },
            ));
            level.add_lod_chunk(
                data.position,
                LODChunkType::Full(data.to_array_chunk(resources.registry.as_ref(), &mut commands)),
            );
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
}

fn shape_chunk<
    const NOISE: usize,
    const HEIGHTMAP: usize,
    const LANDMASS: usize,
    const SQUISH: usize,
>(
    chunk: &mut impl ChunkTrait<BlockId>,
    settings: Arc<ShaperSettings<NOISE, HEIGHTMAP, LANDMASS, SQUISH>>,
    block_id: BlockId,
) -> Heightmap<CHUNK_SIZE> {
    let _my_span = info_span!("shape_chunk", name = "shape_chunk").entered();
    let heightmap_noise = &settings.heightmap_noise;
    let density_noise = &settings.density_noise;
    let landmass_noise = &settings.landmass_noise;
    let squish_noise = &settings.squish_noise;

    const LERP_DISTANCE: u8 = 4;
    const SAMPLE_INTERVAL: usize = (CHUNK_SIZE_U8 / LERP_DISTANCE) as usize;
    const SAMPLES_PER_CHUNK: usize = 1 + SAMPLE_INTERVAL;
    const SAMPLES_PER_CHUNK_U8: u8 = SAMPLES_PER_CHUNK as u8;
    let mut density_samples = [[[0.0; SAMPLES_PER_CHUNK]; SAMPLES_PER_CHUNK]; SAMPLES_PER_CHUNK];
    let mut heightmap = Heightmap([[0.0; CHUNK_SIZE]; CHUNK_SIZE]);

    //use lerp points to make the terrain more sharp, less "blobish"
    for x in 0..SAMPLES_PER_CHUNK {
        for y in 0..SAMPLES_PER_CHUNK {
            for z in 0..SAMPLES_PER_CHUNK {
                let block_pos = chunk.get_block_pos(ChunkIdx::new(
                    x as u8 * LERP_DISTANCE,
                    y as u8 * LERP_DISTANCE,
                    z as u8 * LERP_DISTANCE,
                ));
                density_samples[x][y][z] =
                    density_noise.get_noise3d(block_pos.x, block_pos.y, block_pos.z);
            }
        }
    }

    for x in 0..CHUNK_SIZE_U8 {
        for z in 0..CHUNK_SIZE_U8 {
            let column_pos = chunk.get_block_pos(ChunkIdx::new(x, 0, z));
            let squish = squish_noise.get_noise2d(column_pos.x, column_pos.z);
            let height = squish * heightmap_noise.get_noise2d(column_pos.x, column_pos.z)
                + landmass_noise.get_noise2d(column_pos.x, column_pos.z);
            heightmap.0[x as usize][z as usize] = settings.lower_density.x + height;
            let density_map = ClampedSpline::new([
                Vec2::new(settings.lower_density.x + height, settings.lower_density.y),
                Vec2::new(height, settings.mid_density),
                Vec2::new(
                    crate::util::lerp(0.0, settings.upper_density.x, squish) + height,
                    settings.upper_density.y,
                ),
            ]);
            for y in 0..CHUNK_SIZE_U8 {
                let block_pos = chunk.get_block_pos(ChunkIdx::new(x, y, z));
                let density = trilerp(
                    &density_samples,
                    x as usize,
                    y as usize,
                    z as usize,
                    SAMPLE_INTERVAL,
                );
                if density > density_map.map(block_pos.y) {
                    chunk.set_block(ChunkIdx::new(x, y, z).into(), block_id);
                }
            }
        }
    }
    heightmap
}

pub fn gen_decoration(
    mut chunk: GeneratingChunk,
    heightmap: &Heightmap<CHUNK_SIZE>,
    settings: &DecorationSettings,
) -> (GeneratingChunk, ColumnBiomes<CHUNK_SIZE>) {
    const MID_DEPTH: i32 = 5;
    let mut biome_map = ColumnBiomes([[None; CHUNK_SIZE]; CHUNK_SIZE]);
    for x in 0..CHUNK_SIZE_U8 {
        for z in 0..CHUNK_SIZE_U8 {
            let column_pos = chunk.get_block_pos(ChunkIdx::new(x, 0, z));
            let biome = settings.biomes.sample_id(column_pos);
            biome_map.0[x as usize][z as usize] = biome;
            let biome = settings.biomes.get(biome);
            let target_height = heightmap.0[x as usize][z as usize];
            let mut top_coord = None;
            //find top block (having open air above it)
            for y in (0..CHUNK_SIZE_U8-1).rev() {
                let idx = ChunkIdx::new(x, y, z);
                let block_pos = chunk.get_block_pos(idx);
                if block_pos.y >= target_height {
                    if chunk[idx.to_usize()] == settings.stone && chunk[ChunkIdx::new(x,y+1,z).to_usize()] == BlockId(Id::Empty) {
                        chunk.set_block(idx.into(), biome.topsoil);
                        top_coord = Some(idx);
                        break;
                    }
                }
            }
            //place midsoil under topsoil
            if let Some(top) = top_coord {
                for y in (0.max(top.y as i32-MID_DEPTH)..0.max(top.y as i32-1)).rev() {
                    let idx = ChunkIdx::new(x,y as u8,z).into();
                    if chunk[idx] == settings.stone {
                        chunk.set_block(idx, biome.midsoil);
                    }
                }
            }
        }
    }
    let mut rng = get_next_prng(u32::from_be_bytes(
        (chunk.position.x.wrapping_mul(123979)
            ^ chunk.position.y.wrapping_mul(57891311)
            ^ chunk.position.z.wrapping_mul(7))
        .to_be_bytes(),
    ) as u64);
    for generator in &settings.ores {
        rng = get_next_prng(rng);
        if let Some(mut idx) = generator.get_ore_placement(rng) {
            rng = get_next_prng(rng);
            let vein_size =
                generator.vein_min + (rng as u32 % (generator.vein_max - generator.vein_min));
            for _ in 0..vein_size {
                if generator.can_replace.contains(&chunk[idx]) {
                    chunk.set_block(idx.into(), generator.ore_block);
                }
                rng = get_next_prng(rng);
                idx = idx.offset(crate::util::Direction::from(rng));
            }
        }
    }
    (chunk, biome_map)
}
