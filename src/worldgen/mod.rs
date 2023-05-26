use std::sync::Arc;

use bevy::prelude::*;
use bracket_noise::prelude::*;

use crate::{world::{chunk::ChunkCoord, LevelSystemSet}, util::Spline};

use self::worldgen::{ChunkNeedsGenerated, WorldGenSettings};

pub mod worldgen;

const QUEUE_GEN_TIME_BUDGET_MS: u128 = 10;
const ADD_TIME_BUDGET_MS: u128 = 10;

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        let build_gen_system = || {
            move |query: Query<(Entity, &ChunkCoord), With<ChunkNeedsGenerated>>,
                  commands: Commands| {
                worldgen::queue_generating(query, Arc::new(create_settings(8008135)), commands)
            }
        };
        app.add_systems((worldgen::poll_gen_queue,build_gen_system()).in_set(LevelSystemSet::Main));
    }
}

fn create_settings(seed: u64) -> WorldGenSettings {
    WorldGenSettings {
        noise: create_noise(seed),
        density_by_height: Spline::new(&[Vec2::new(0.0,1.0), Vec2::new(24.0,-0.5), Vec2::new(48.0, -1.0)])
    }
}

fn create_noise(seed: u64) -> FastNoise {
    let mut noise = FastNoise::seeded(seed);
    noise.set_noise_type(NoiseType::SimplexFractal);
    noise.set_frequency(0.02);
    noise.set_fractal_octaves(3);
    //amp multiplier
    noise.set_fractal_gain(0.7);
    //freq multiplier
    noise.set_fractal_lacunarity(2.);
    noise
}