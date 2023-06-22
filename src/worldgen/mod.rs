use std::sync::Arc;

use bevy::{prelude::*};
use bracket_noise::prelude::*;

use crate::{world::{chunk::ChunkCoord, LevelSystemSet, Level}, util::{Spline, SplineNoise, get_next_prng}};

mod generator;
pub use generator::{ChunkNeedsGenerated, GeneratedChunk, GeneratedLODChunk, ShapingTask, LODShapingTask, ShaperSettings};

use self::{structures::{StructureGenerationSettings, trees::get_short_tree}, generator::GenSmallStructureTask};

pub mod structures;

const QUEUE_GEN_TIME_BUDGET_MS: u128 = 10;
const ADD_TIME_BUDGET_MS: u128 = 10;

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        let build_gen_system = || {
            move |query: Query<(Entity, &ChunkCoord, &ChunkNeedsGenerated)>,
                  commands: Commands| {
                generator::queue_generating(query, Arc::new(create_shaper_settings(8008135)), commands)
            }
        };
        let build_poll_system = || {
            move | shaping_query: Query<(Entity, &mut Transform, &mut ShapingTask)>,
                    structure_query: Query<(Entity, &mut GenSmallStructureTask)>,
                    level: Res<Level>,
                  commands: Commands| {
                generator::poll_gen_queue(Arc::new(create_structure_settings(424242)), commands, shaping_query, structure_query, level)
            }
        };
        app.add_systems((build_poll_system(),build_gen_system(), generator::poll_gen_lod_queue).in_set(LevelSystemSet::LoadingAndMain));
    }
}

fn create_shaper_settings(seed: u64) -> ShaperSettings {
    ShaperSettings {
        noise: create_shaper_noise(seed),
        upper_density: Vec2::new(0.0,-1.0),
        heightmap_noise: create_heighmap_noise(seed^0xCAFEBABEDEAFBEEF), //don't want the seeds to be the same
        mid_density: 0.0,
        base_density: Vec2::new(-100.0,1.0)
    }
}

fn create_shaper_noise(seed: u64) -> SplineNoise {
    let mut noise = FastNoise::seeded(seed);
    noise.set_noise_type(NoiseType::SimplexFractal);
    noise.set_fractal_type(FractalType::RigidMulti);
    noise.set_frequency(0.002);
    noise.set_fractal_octaves(3);
    //amp multiplier
    noise.set_fractal_gain(0.5);
    //freq multiplier
    noise.set_fractal_lacunarity(3.0);
    let spline = Spline::new(&[Vec2::new(-1.0,-1.0), Vec2::new(1.0,1.0)]);
    SplineNoise {
        noise,
        spline
    }
}

fn create_heighmap_noise(seed: u64) -> SplineNoise {
    let mut noise = FastNoise::seeded(seed);
    noise.set_noise_type(NoiseType::SimplexFractal);
    noise.set_fractal_type(FractalType::Billow);
    noise.set_frequency(0.002);
    noise.set_fractal_octaves(1);
    //amp multiplier
    noise.set_fractal_gain(0.5);
    //freq multiplier
    noise.set_fractal_lacunarity(3.0);
    let spline = Spline::new(&[Vec2::new(-1.0,-50.0), Vec2::new(0.0, 0.0), Vec2::new(1.0,250.0)]);
    SplineNoise {
        noise,
        spline
    }
}

fn create_structure_settings(mut seed: u64) -> StructureGenerationSettings {
    let mut noise = FastNoise::seeded(seed);
    noise.set_noise_type(NoiseType::Value);
    noise.set_frequency(843580.97854);
    let structures = vec![get_short_tree(get_next_seed(&mut seed))];

    StructureGenerationSettings { rolls_per_chunk: 5, structures, placement_noise: noise}
}

fn get_next_seed(seed: &mut u64) -> u64 {
    *seed = get_next_prng::<16>(*seed);
    *seed
}