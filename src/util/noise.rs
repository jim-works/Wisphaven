use bracket_noise::prelude::*;

use super::Spline;

pub struct SplineNoise {
    pub noise: FastNoise,
    pub spline: Spline
}

impl SplineNoise {
    pub fn get_noise3d(&self, x: f32, y: f32, z: f32) -> f32 {
        self.spline.map(self.noise.get_noise3d(x, y, z))
    }
    pub fn get_noise2d(&self, x: f32, y: f32) -> f32 {
        self.spline.map(self.noise.get_noise(x, y))
    }
}