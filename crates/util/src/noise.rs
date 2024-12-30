use std::ops::Range;

use bevy::math::IVec3;
use bracket_noise::prelude::*;
use rand::Rng;
use rand_distr::Uniform;

use super::spline::Spline;

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

pub trait ToSeed {
    fn to_seed(&self) -> u64;
}

//xqo generator
//todo: support 64 bit
pub fn get_next_prng(input: u64) -> u64 {
    let input = input as u32;
    let state = (input | 1) ^ input.wrapping_mul(input);
    let word =
        277803737_u32.wrapping_mul(state.rotate_right((state >> 28).wrapping_add(4)) ^ state);
    ((word >> 22) ^ word) as u64
}

pub fn prng_3d(seed: u64, pos: IVec3) -> IVec3 {
    let offset = 104729_i64.wrapping_mul(seed as i64)
        ^ 224737_i64.wrapping_mul(pos.x as i64)
        ^ 350377_i64.wrapping_mul(pos.y as i64)
        ^ 479909_i64.wrapping_mul(pos.z as i64);
    IVec3::new(
        (pos.x as i64).wrapping_mul(offset) as i32,
        (pos.y as i64).wrapping_mul(offset) as i32,
        (pos.z as i64).wrapping_mul(offset) as i32,
    )
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
    let diff = rng % (range.end - range.start);
    range.start + diff
}

pub fn prng_select<T>(seed: u64, slice: &[T]) -> Option<&T> {
    slice.get(get_next_prng(seed) as usize % slice.len())
}
