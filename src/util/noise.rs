use std::ops::Range;

use bracket_noise::prelude::*;
use rand::Rng;
use rand_distr::Uniform;

use crate::world::{chunk::ChunkCoord, BlockCoord};

use super::Spline;

pub struct SplineNoise<const S: usize> {
    pub noise: FastNoise,
    pub spline: Spline<S>,
}

impl<const S: usize> SplineNoise<S> {
    pub fn get_noise3d(&self, x: f32, y: f32, z: f32) -> f32 {
        self.spline.map(self.noise.get_noise3d(x, y, z))
    }
    pub fn get_noise2d(&self, x: f32, y: f32) -> f32 {
        self.spline.map(self.noise.get_noise(x, y))
    }
}
fn rot(x: u64) -> u64 {
    (x << 16) | (x >> 16)
}

pub trait ToSeed {
    fn to_seed(&self) -> u64;
}

impl ToSeed for ChunkCoord {
    fn to_seed(&self) -> u64 {
        let upper = u32::from_le_bytes(self.x.wrapping_mul(123979).to_le_bytes()) as u64;
        let lower = u32::from_le_bytes((self.y.wrapping_mul(57891311)^self.z.wrapping_mul(7)).to_le_bytes()) as u64;
        upper << 32 | lower
    }
}

impl ToSeed for BlockCoord {
    fn to_seed(&self) -> u64 {
        let upper = u32::from_le_bytes(self.x.wrapping_mul(57891311).to_le_bytes()) as u64;
        let lower = u32::from_le_bytes((self.y.wrapping_mul(13)^ self.z.wrapping_mul(123979)).to_le_bytes()) as u64;
        upper << 32 | lower
    }
}

//xqo generator
//todo: support 64 bit
pub fn get_next_prng(input: u64) -> u64
{
    let input = input as u32;
    let state = (input | 1) ^ input.wrapping_mul(input);
    let word = 277803737_u32.wrapping_mul(state.rotate_right((state >> 28).wrapping_add(4)) ^ state);
    return ((word >> 22) ^ word) as u64;
}

pub fn mut_next_prng(input: &mut u64) -> u64 {
    *input = get_next_prng(*input);
    *input
}

pub fn sample_range(range: Range<f32>, rng: &mut impl Rng) -> f32 {
    rng.sample(Uniform::new(range.start, range.end))
}

pub fn prng_sample_range(range: Range<u64>, seed: u64) -> u64 {
    let rng = get_next_prng(seed);
    let diff = rng % (range.end-range.start);
    range.start + diff
}