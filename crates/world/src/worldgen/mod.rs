use std::sync::Arc;

use bevy::prelude::*;
use bracket_noise::prelude::*;

use crate::{level::Level, BlockId, BlockName, BlockResources, LevelLoadState, LevelSystemSet};
use util::{noise::get_next_prng, noise::SplineNoise, spline::Spline};

mod generator;
pub mod pipeline;
use pipeline::{ChunkNeedsGenerated, GeneratedChunk, ShaperSettings};

use self::{biomes::UsedBiomeMap, pipeline::OreGenerator};

pub mod biomes;
pub mod structures;

const QUEUE_GEN_TIME_BUDGET_MS: u128 = 10;
const ADD_TIME_BUDGET_MS: u128 = 10;

pub const HEIGHTMAP: usize = 6;
pub const LANDMASS: usize = 5;
pub const DENSITY: usize = 2;
pub const SQUISH: usize = 7;

pub type UsedShaperResources =
    ShaperResources<{ DENSITY }, { HEIGHTMAP }, { LANDMASS }, { SQUISH }>;

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                pipeline::poll_shaping_task,
                pipeline::poll_decoration_waiters,
                pipeline::poll_decoration_task,
                pipeline::poll_structure_waiters,
                pipeline::poll_structure_task,
                pipeline::queue_generating::<DENSITY, HEIGHTMAP, LANDMASS, SQUISH>,
                pipeline::poll_gen_lod_queue,
            )
                .in_set(LevelSystemSet::LoadingAndMain),
        )
        .add_systems(
            OnEnter(LevelLoadState::Loading),
            (create_shaper_settings, create_decoration_settings),
        );
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Ord, PartialOrd)]
pub enum GenerationPhase {
    Shaped = 0,
    Decorated = 1,
    Structured = 2,
}

#[derive(Resource)]
pub struct ShaperResources<const D: usize, const H: usize, const L: usize, const S: usize>(
    pub Arc<ShaperSettings<D, H, L, S>>,
);

#[derive(Resource)]
pub struct DecorationResources(pub Arc<DecorationSettings>);

pub struct DecorationSettings {
    pub biomes: UsedBiomeMap,
    //white noise for ores
    pub ore_noise: FastNoise,
    pub stone: BlockId,
    pub ores: Vec<OreGenerator>,
}

fn create_shaper_settings(mut commands: Commands, level: Res<Level>) {
    let mut seed = level.seed ^ 0xABDFACDFAEDFA0DF;
    let settings = ShaperSettings {
        density_noise: create_density_noise(seed),
        landmass_noise: create_landmass_noise(get_next_seed(&mut seed)),
        squish_noise: create_squish_noise(get_next_seed(&mut seed)),
        //x = terrain height, y = density threshold to place a block
        //this is the maximum height, but an offset: heightmap_noise+upper_density.x = the highest control point on the spline
        upper_density: Vec2::new(25.0, 1.0),
        //this is the middle height: which basically controls the
        heightmap_noise: create_heightmap_noise(get_next_seed(&mut seed)), //don't want the seeds to be the same
        mid_density: 0.0,
        //this is the minimum height, but an offset: heightmap_noise+lower_density.x = the lowest control point on the spline
        lower_density: Vec2::new(-100.0, -0.2),
    };
    commands.insert_resource(ShaperResources(Arc::new(settings)));
}

fn create_density_noise(seed: u64) -> SplineNoise<DENSITY> {
    let mut noise = FastNoise::seeded(seed);
    noise.set_noise_type(NoiseType::SimplexFractal);
    noise.set_fractal_type(FractalType::RigidMulti);
    noise.set_frequency(0.003);
    noise.set_fractal_octaves(4);
    //amp multiplier
    noise.set_fractal_gain(0.4);
    //freq multiplier
    noise.set_fractal_lacunarity(3.0);
    let spline = Spline::new([Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)]);
    SplineNoise { noise, spline }
}

//decides target height for a column
fn create_heightmap_noise(seed: u64) -> SplineNoise<HEIGHTMAP> {
    let mut noise = FastNoise::seeded(seed);
    noise.set_noise_type(NoiseType::SimplexFractal);
    noise.set_fractal_type(FractalType::FBM);
    noise.set_frequency(0.001);
    noise.set_fractal_octaves(3);
    //amp multiplier
    noise.set_fractal_gain(0.5);
    //freq multiplier
    noise.set_fractal_lacunarity(3.0);
    let spline = Spline::new([
        Vec2::new(-0.6, -100.0),
        Vec2::new(-0.3, -50.0),
        Vec2::new(-0.2, -20.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(0.3, 20.0),
        Vec2::new(0.5, 100.0),
    ]);
    SplineNoise { noise, spline }
}

fn create_landmass_noise(seed: u64) -> SplineNoise<LANDMASS> {
    let mut noise = FastNoise::seeded(seed);
    //negative values indicate ocean, positive indicate land
    noise.set_noise_type(NoiseType::SimplexFractal);
    noise.set_fractal_type(FractalType::FBM);
    noise.set_frequency(0.00003);
    noise.set_fractal_octaves(3);
    noise.set_fractal_gain(0.5);
    noise.set_fractal_lacunarity(3.0);
    let spline = Spline::new([
        Vec2::new(-0.5, -200.0), //deep ocean
        Vec2::new(-0.2, -25.0),  //shallow ocean
        Vec2::new(-0.1, 0.0),    //lower land
        Vec2::new(0.1, 20.0),    //normal land
        Vec2::new(0.3, 200.0),   //continent-defining mountains
    ]);
    SplineNoise { noise, spline }
}

//decides how flat the terrain is. multipler on heightmap noise
fn create_squish_noise(seed: u64) -> SplineNoise<SQUISH> {
    let mut noise = FastNoise::seeded(seed);
    noise.set_noise_type(NoiseType::SimplexFractal);
    noise.set_fractal_type(FractalType::FBM);
    noise.set_frequency(0.0005);
    noise.set_fractal_octaves(2);
    //amp multiplier
    noise.set_fractal_gain(0.5);
    //freq multiplier
    noise.set_fractal_lacunarity(3.0);
    let spline = Spline::new([
        Vec2::new(-0.4, 2.0),
        Vec2::new(-0.3, 0.3),
        Vec2::new(-0.2, 1.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.1, 0.2),
        Vec2::new(0.3, 0.1),
        Vec2::new(0.4, 1.2),
    ]);
    SplineNoise { noise, spline }
}

fn create_decoration_settings(
    level: Res<Level>,
    mut commands: Commands,
    resources: Res<BlockResources>,
) {
    let mut seed = level.seed ^ 0x6287192746;

    let mut ore_noise = FastNoise::seeded(get_next_seed(&mut seed));
    ore_noise.set_noise_type(NoiseType::Value);
    ore_noise.set_frequency(132671324.0);

    commands.insert_resource(DecorationResources(Arc::new(DecorationSettings {
        biomes: UsedBiomeMap::default(&resources.registry, seed),
        ore_noise,
        stone: resources.registry.get_id(&BlockName::core("stone")),
        ores: vec![OreGenerator {
            ore_block: resources.registry.get_id(&BlockName::core("ruby_ore")),
            can_replace: vec![resources.registry.get_id(&BlockName::core("stone"))],
            rarity: (0, 1),
            vein_min: 10,
            vein_max: 20,
        }],
    })))
}

fn get_next_seed(seed: &mut u64) -> u64 {
    *seed = get_next_prng(*seed);
    *seed
}
