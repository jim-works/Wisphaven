use std::ops::Range;

use bevy::prelude::{Vec2, Vec3};
use bracket_noise::prelude::*;

use crate::{
    util::{get_next_prng, Buckets, Spline, SplineNoise, ToSeed},
    world::{
        chunk::{ChunkIdx, GeneratingChunk, CHUNK_SIZE_U64},
        BlockCoord, BlockId, BlockName, BlockRegistry,
    },
};

use super::{
    get_next_seed,
    structures::{
        trees::{get_cactus, get_short_tree},
        LargeStructureGenerator, StructureGenerator, fauna::FauanaGenerator,
    },
};

pub const TEMP: usize = 2;
pub const HUMID: usize = 2;
pub const FUNKY: usize = 2;

pub type UsedBiomeMap = BiomeMap<{ TEMP }, { HUMID }, { FUNKY }>;

pub struct Biome {
    pub topsoil: BlockId,
    pub midsoil: BlockId,
    pub soil_depth: u8, //must be less than CHUNK_SIZE
    pub fallback_generator: Option<BiomeStructureGenerator>,
}

pub struct BiomeMap<const TEMP: usize, const HUMID: usize, const FUNKY: usize> {
    //maps to index in biomes array based on temperature and humidity
    pub map: Buckets<Buckets<Buckets<usize>>>,
    pub biomes: Vec<Biome>,
    pub default_biome: usize,
    //2d temperature for biome placement
    pub temperature_noise: SplineNoise<TEMP>,
    //2d humidity for biome placement
    pub humidity_noise: SplineNoise<HUMID>,
    //3d "funkiness" for biome placement,
    pub funky_noise: SplineNoise<FUNKY>,
}

impl<const TEMP: usize, const HUMID: usize, const FUNKY: usize> BiomeMap<TEMP, HUMID, FUNKY> {
    pub fn get_id(&self, heightmap: f32, temp: f32, humid: f32) -> Option<usize> {
        self.map
            .map(heightmap)
            .and_then(|b| b.map(temp).and_then(|b| b.map(humid)))
            .copied()
    }
    pub fn get(&self, id: Option<usize>) -> &Biome {
        id.map(|x| {
            self.biomes
                .get(x)
                .unwrap_or(&self.biomes[self.default_biome])
        })
        .unwrap_or(&self.biomes[self.default_biome])
    }
    pub fn sample(&self, heightmap: f32, pos: Vec3) -> &Biome {
        let temp = self.temperature_noise.get_noise2d(pos.x, pos.z);
        let humid = self.humidity_noise.get_noise2d(pos.x, pos.z);
        self.get(self.get_id(heightmap, temp, humid))
    }
    pub fn sample_id(&self, heightmap: f32, pos: Vec3) -> Option<usize> {
        let temp = self.temperature_noise.get_noise2d(pos.x, pos.z);
        let humid = self.humidity_noise.get_noise2d(pos.x, pos.z);
        self.get_id(heightmap, temp, humid)
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
            soil_depth: 4,
            fallback_generator: Some(BiomeStructureGenerator {
                structures: vec![BiomeStructure {
                    gen: get_short_tree(
                        get_next_seed(&mut seed),
                        Range { start: 4, end: 8 },
                        Range { start: 8, end: 15 },
                        0.5,
                        registry,
                    ),
                    rolls_per_chunk: 5,
                },
                BiomeStructure {
                    gen: Box::new(FauanaGenerator {to_spawn: registry.get_id(&BlockName::core("lily")), spawn_on: registry.get_id(&BlockName::core("grass"))}),
                    rolls_per_chunk: 100,
                }]
            }),
        };
        let desert = Biome {
            topsoil: registry.get_id(&BlockName::core("sand")),
            midsoil: registry.get_id(&BlockName::core("sand")),
            soil_depth: 15,
            fallback_generator: Some(BiomeStructureGenerator {
                structures: vec![BiomeStructure {
                    gen: get_cactus(
                        get_next_seed(&mut seed),
                        Range { start: 3, end: 8 },
                        0.5,
                        2,
                        4,
                        registry,
                    ),
                    rolls_per_chunk: 5,
                }]
            }),
        };
        let rocks = Biome {
            topsoil: registry.get_id(&BlockName::core("stone")),
            midsoil: registry.get_id(&BlockName::core("stone")),
            soil_depth: 0,
            fallback_generator: None,
        };
        let snowy_mountains = Biome {
            topsoil: registry.get_id(&BlockName::core("snow_sheet")),
            midsoil: registry.get_id(&BlockName::core("snow")),
            soil_depth: 2,
            fallback_generator: None,
        };
        Self {
            biomes: vec![meadow, desert, snowy_mountains, rocks],
            map: Buckets::new(vec![
                (
                    150.0,
                    Buckets::new(vec![
                        (-0.1, Buckets::new(vec![(0.1, 0), (0.3, 1)])),
                        (0.1, Buckets::new(vec![(0.0, 0), (0.3, 1)])),
                    ]),
                ),
                (
                    200.0,
                    Buckets::new(vec![
                        (-0.1, Buckets::new(vec![(-0.2, 3), (0.1, 0), (0.3, 1)])),
                        (0.1, Buckets::new(vec![(-0.3, 3), (0.0, 0), (0.3, 1)])),
                    ]),
                ),
                (
                    250.0,
                    Buckets::new(vec![
                        (-0.1, Buckets::new(vec![(0.2, 2), (0.3, 0), (0.4, 1)])),
                        (0.1, Buckets::new(vec![(0.0, 3), (0.1, 0), (0.3, 1)])),
                    ]),
                ),
                (
                    300.0,
                    Buckets::new(vec![
                        (0.0, Buckets::new(vec![(0.0, 2)])),
                    ]),
                ),
            ]),
            default_biome: 0,
            temperature_noise,
            humidity_noise,
            funky_noise,
        }
    }
}

pub struct BiomeStructure {
    gen: Box<dyn StructureGenerator + Sync + Send>,
    pub rolls_per_chunk: i32,
}

pub struct BiomeStructureGenerator {
    pub structures: Vec<BiomeStructure>
}

impl StructureGenerator for BiomeStructureGenerator {
    fn rarity(&self) -> f32 {
        0.0
    }

    fn generate(
        &self,
        buffer: &mut crate::world::BlockBuffer<BlockId>,
        world_pos: BlockCoord,
        _: ChunkIdx,
        chunk: &GeneratingChunk,
    ) -> bool {
        let mut rng = get_next_prng(world_pos.to_seed());
        for structure in &self.structures {
            for _ in 0..structure.rolls_per_chunk {
                let x = get_next_prng(rng);
                let y = get_next_prng(x);
                let z = get_next_prng(y);
                rng = get_next_prng(z);
                let pos = ChunkIdx::new(
                    (x % CHUNK_SIZE_U64) as u8,
                    (y % CHUNK_SIZE_U64) as u8,
                    (z % CHUNK_SIZE_U64) as u8,
                );
                structure.gen.generate(
                    buffer,
                    BlockCoord::from(chunk.position)
                        + BlockCoord::new(pos.x as i32, pos.y as i32, pos.z as i32),
                    pos,
                    chunk,
                );
            }
        }
        true
    }
}

impl LargeStructureGenerator for BiomeStructureGenerator {
    fn setup(&mut self, _world_pos: crate::world::BlockCoord) {}
}
