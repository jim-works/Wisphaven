use bevy::prelude::{Vec2, Vec3};
use bracket_noise::prelude::*;

use crate::{
    util::{Buckets, SplineNoise, Spline},
    world::{BlockId, BlockName, BlockRegistry},
};

use super::get_next_seed;

pub const TEMP: usize = 2;
pub const HUMID: usize = 2;
pub const FUNKY: usize = 2;

pub type UsedBiomeMap = BiomeMap<{TEMP},{HUMID},{FUNKY}>;

pub struct Biome {
    pub topsoil: BlockId,
    pub midsoil: BlockId,
}

pub struct BiomeMap<const TEMP: usize, const HUMID: usize, const FUNKY: usize> {
    //maps to index in biomes array based on temperature and humidity
    pub map: Buckets<Buckets<usize>>,
    pub biomes: Vec<Biome>,
    pub default_biome: Biome,
    //2d temperature for biome placement
    pub temperature_noise: SplineNoise<TEMP>,
    //2d humidity for biome placement
    pub humidity_noise: SplineNoise<HUMID>,
    //3d "funkiness" for biome placement,
    pub funky_noise: SplineNoise<FUNKY>
}

impl<const TEMP: usize, const HUMID: usize, const FUNKY: usize> BiomeMap<TEMP,HUMID,FUNKY> {
    pub fn get(&self, temp: f32, humid: f32) -> &Biome {
        self.map
            .map(temp)
            .and_then(|b| b.map(humid))
            .map(|idx| self.biomes.get(*idx).unwrap_or(&self.default_biome))
            .unwrap_or(&self.default_biome)
    }
    pub fn sample(&self, pos: Vec3) -> &Biome {
        let temp = self.temperature_noise.get_noise2d(pos.x, pos.z);
        let humid = self.humidity_noise.get_noise2d(pos.x, pos.z);
        self.get(temp, humid)
    }
}

impl UsedBiomeMap {
    pub fn default(registry: &BlockRegistry, mut seed: u64) -> Self {
        let mut noise = FastNoise::seeded(seed);
        noise.set_noise_type(NoiseType::SimplexFractal);
        noise.set_frequency(0.001);
        noise.set_fractal_octaves(2);
        noise.set_fractal_gain(0.5);
        noise.set_fractal_lacunarity(3.0);
        let temperature_noise = SplineNoise {
            noise,
            spline: Spline::new([Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)]),
        };
    
        let mut noise = FastNoise::seeded(get_next_seed(&mut seed));
        noise.set_noise_type(NoiseType::SimplexFractal);
        noise.set_frequency(0.001);
        noise.set_fractal_octaves(2);
        noise.set_fractal_gain(0.5);
        noise.set_fractal_lacunarity(3.0);
        let humidity_noise = SplineNoise {
            noise,
            spline: Spline::new([Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)]),
        };
    
        let mut noise = FastNoise::seeded(get_next_seed(&mut seed));
        noise.set_noise_type(NoiseType::SimplexFractal);
        noise.set_frequency(0.001);
        noise.set_fractal_octaves(2);
        noise.set_fractal_gain(0.5);
        noise.set_fractal_lacunarity(3.0);
        let funky_noise = SplineNoise {
            noise,
            spline: Spline::new([Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)]),
        };

        let meadow = Biome {
            topsoil: registry.get_id(&BlockName::core("grass")),
            midsoil: registry.get_id(&BlockName::core("dirt")),
        };
        let desert = Biome {
            topsoil: registry.get_id(&BlockName::core("sand")),
            midsoil: registry.get_id(&BlockName::core("sand")),
        };
        let rocks = Biome {
            topsoil: registry.get_id(&BlockName::core("stone")),
            midsoil: registry.get_id(&BlockName::core("stone")),
        };
        Self {
            biomes: vec![meadow, desert],
            map: Buckets::new(vec![
                (-0.1, Buckets::new(vec![(0.1, 0), (0.3, 1)])),
                (0.1, Buckets::new(vec![(0.0, 0), (0.3, 1)])),
            ]),
            default_biome: rocks,
            temperature_noise,
            humidity_noise,
            funky_noise
        }
    }
}