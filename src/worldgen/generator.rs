use bracket_noise::prelude::FastNoise;
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
    UsedDecorationResources, UsedDecorationSettings, UsedShaperResources, ADD_TIME_BUDGET_MS,
    QUEUE_GEN_TIME_BUDGET_MS,
};

#[derive(Component)]
pub enum ChunkNeedsGenerated {
    Full,
    Lod(u8),
}

//task to generate the overall shape of the terrain
#[derive(Component)]
pub struct ShapingTask {
    pub task: Task<GeneratingChunk>,
}

//task to generate small structures (trees, buildings, etc)
#[derive(Component)]
pub struct DecorationTask {
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

pub struct DecorationSettings<const TEMP: usize, const HUMID: usize, const FUNKY: usize> {
    //2d temperature for biome placement
    pub temperature_noise: SplineNoise<TEMP>,
    //2d humidity for biome placement
    pub humidity_noise: SplineNoise<HUMID>,
    //3d "funkiness" for biome placement,
    pub funky_noise: SplineNoise<FUNKY>,
    //white noise for ores
    pub ore_noise: FastNoise,
    pub humid_block: BlockId,
    pub stone: BlockId,
    pub ores: Vec<OreGenerator>,
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
                        shape_chunk(&mut chunk, gen_noise, id);
                        chunk
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

pub fn poll_gen_queue(
    structure_resources: Res<StructureResources>,
    decor_resources: Res<UsedDecorationResources>,
    mut commands: Commands,
    mut shaping_query: Query<(Entity, &mut Transform, &mut ShapingTask)>,
    mut decor_query: Query<(Entity, &mut DecorationTask)>,
    mut structure_query: Query<(Entity, &mut GenSmallStructureTask)>,
    resources: Res<BlockResources>,
    level: Res<Level>,
) {
    let _my_span = info_span!("poll_gen_queue", name = "poll_gen_queue").entered();
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    for (entity, mut tf, mut task) in shaping_query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            tf.translation = data.position.to_vec3();
            let settings = decor_resources.0.clone();
            commands
                .entity(entity)
                .remove::<ShapingTask>()
                .insert(DecorationTask {
                    task: pool.spawn(async move { gen_decoration(data, settings.as_ref()) }),
                });
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
    for (entity, mut task) in decor_query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            let settings = structure_resources.settings.clone();
            commands
                .entity(entity)
                .remove::<DecorationTask>()
                .insert(GenSmallStructureTask {
                    task: pool
                        .spawn(async move { structures::gen_small_structures(data, settings) }),
                });
            let duration = Instant::now().duration_since(now).as_millis();
            if duration > ADD_TIME_BUDGET_MS {
                break;
            }
        }
    }
    for (entity, mut task) in structure_query.iter_mut() {
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
                .remove::<GenSmallStructureTask>()
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
    mut level: ResMut<Level>,
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
) {
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
}

pub fn gen_decoration(
    mut chunk: GeneratingChunk,
    settings: &UsedDecorationSettings,
) -> GeneratingChunk {
    for x in 0..CHUNK_SIZE_U8 {
        for z in 0..CHUNK_SIZE_U8 {
            let column_pos = chunk.get_block_pos(ChunkIdx::new(x, 0, z));
            let humidity = settings
                .humidity_noise
                .get_noise2d(column_pos.x, column_pos.z);
            if humidity > 0.0 {
                for y in 0..CHUNK_SIZE_U8 {
                    let idx = ChunkIdx::new(x, y, z).into();
                    if chunk[idx] == settings.stone {
                        chunk.set_block(idx, settings.humid_block)
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
    chunk
}
