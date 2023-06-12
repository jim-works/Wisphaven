use bracket_noise::prelude::*;

use super::Spline;

pub struct SplineNoise {
    pub noise: FastNoise,
    pub spline: Spline,
}

impl SplineNoise {
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
//https://en.wikipedia.org/wiki/Linear_congruential_generator
pub fn get_next_prng<const ITERATIONS: u64>(curr: u64) -> u64 {
    let mut seed = curr;
    for i in 0..ITERATIONS {
        seed = seed.wrapping_mul(541).wrapping_add(i);
        seed = rot(seed);
        seed = seed.wrapping_mul(809).wrapping_add(i);
        seed = rot(seed);
        seed = seed.wrapping_mul(673).wrapping_add(i);
        seed = rot(seed);
    }
    return seed;
}
