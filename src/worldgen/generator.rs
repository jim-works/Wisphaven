use std::sync::Arc;

use crate::{

    util::{get_next_prng, trilerp, ClampedSpline},
    world::{
        chunk::*, BlockId, BlockType, Id
    }, worldgen::pipeline::ColumnBiomes,
};
use bevy::prelude::*;

use super::{DecorationSettings, ShaperSettings, pipeline::Heightmap};


pub fn shape_chunk<
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
    chunk: &mut GeneratingChunk,
    chunk_above: &ChunkType, //should not be ungenerated
    heightmap: &Heightmap<CHUNK_SIZE>,
    settings: &DecorationSettings,
) -> ColumnBiomes<CHUNK_SIZE> {
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
            let top_block_idx = ChunkIdx::new(x, CHUNK_SIZE_U8 - 1, z);
            if chunk[top_block_idx.to_usize()] == settings.stone
                && chunk.get_block_pos(top_block_idx).y >= target_height
            {
                //top block of the chunk is sufficiently high, so check if there's air in the chunk above
                let air = match chunk_above {
                    ChunkType::Ungenerated(_) => unreachable!(),
                    ChunkType::Generating(_, chunk) => {
                        chunk[ChunkIdx::new(x, 0, z).to_usize()] == BlockId(Id::Empty)
                    }
                    ChunkType::Full(chunk) => {
                        chunk[ChunkIdx::new(x, 0, z).to_usize()] == BlockType::Empty
                    }
                };
                if air {
                    chunk.set_block(top_block_idx.into(), biome.topsoil);
                    top_coord = Some(top_block_idx);
                }
            }
            if top_coord.is_none() {
                for y in (0..CHUNK_SIZE_U8 - 1).rev() {
                    let idx = ChunkIdx::new(x, y, z);
                    let block_pos = chunk.get_block_pos(idx);
                    if block_pos.y >= target_height {
                        if chunk[idx.to_usize()] == settings.stone
                            && chunk[ChunkIdx::new(x, y + 1, z).to_usize()] == BlockId(Id::Empty)
                        {
                            chunk.set_block(idx.into(), biome.topsoil);
                            top_coord = Some(idx);
                            break;
                        }
                    }
                }
            }
            //place midsoil under topsoil
            if let Some(top) = top_coord {
                for y in (0.max(top.y as i32 - MID_DEPTH)..0.max(top.y as i32 - 1)).rev() {
                    let idx = ChunkIdx::new(x, y as u8, z).into();
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
    biome_map
}
