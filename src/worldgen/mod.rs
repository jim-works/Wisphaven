use std::sync::Arc;

use bevy::prelude::*;
use bracket_noise::prelude::*;

use crate::world::chunk::ChunkCoord;

use self::worldgen::ChunkNeedsGenerated;

pub mod worldgen;

const QUEUE_GEN_TIME_BUDGET_MS: u128 = 1;
const ADD_TIME_BUDGET_COUNT: u128 = 1;

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        let build_gen_system = || {
            move |query: Query<(Entity, &ChunkCoord), With<ChunkNeedsGenerated>>,
                  commands: Commands| {
                worldgen::queue_generating(query, Arc::new(create_noise(8008135)), commands)
            }
        };
        app.add_system(worldgen::poll_gen_queue)
            .add_system(build_gen_system());
    }
}

fn create_noise(seed: u64) -> FastNoise {
    let mut noise = FastNoise::seeded(seed);
    noise.set_noise_type(NoiseType::SimplexFractal);
    noise.set_frequency(0.02);
    noise.set_fractal_octaves(4);
    //amp multiplier
    noise.set_fractal_gain(0.7);
    //freq multiplier
    noise.set_fractal_lacunarity(2.);
    noise
}