use bracket_noise::prelude::*;

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

//xqo generator
//todo: support 64 bit
pub fn get_next_prng(input: u64) -> u64
{
    let input = input as u32;
    let state = (input | 1) ^ input.wrapping_mul(input);
    let word = 277803737_u32.wrapping_mul(state.rotate_right((state >> 28).wrapping_add(4)) ^ state);
    return ((word >> 22) ^ word) as u64;
}