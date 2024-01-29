mod direction;
use std::time::Duration;

pub use direction::*;

mod spline;
use itertools::Itertools;
pub use spline::*;

mod noise;
pub use noise::*;

pub mod l_system;

mod numerical_traits;
pub use numerical_traits::*;

pub mod bevy_utils;
pub mod controls;
pub mod iterators;
pub mod palette;
pub mod physics;
pub mod plugin;
pub mod string;

use bevy::{
    prelude::{Deref, DerefMut, Vec3},
    time::Timer,
};

use rand::prelude::*;
use rand_distr::StandardNormal;

#[cfg(test)]
mod test;

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

pub fn trilerp<const X: usize, const Y: usize, const Z: usize>(
    samples: &[[[f32; X]; Y]; Z],
    x: usize,
    y: usize,
    z: usize,
    sample_interval: usize,
) -> f32 {
    let index_x = x % sample_interval;
    let index_y = y % sample_interval;
    let index_z = z % sample_interval;

    let factor_x = index_x as f32 / sample_interval as f32;
    let factor_y = index_y as f32 / sample_interval as f32;
    let factor_z = index_z as f32 / sample_interval as f32;

    let point = Vec3::new(factor_x, factor_y, factor_z);

    let v000 = samples[x / sample_interval][y / sample_interval][z / sample_interval];
    let v001 = samples[x / sample_interval][y / sample_interval][z / sample_interval + 1];
    let v010 = samples[x / sample_interval][y / sample_interval + 1][z / sample_interval];
    let v011 = samples[x / sample_interval][y / sample_interval + 1][z / sample_interval + 1];
    let v100 = samples[x / sample_interval + 1][y / sample_interval][z / sample_interval];
    let v101 = samples[x / sample_interval + 1][y / sample_interval][z / sample_interval + 1];
    let v110 = samples[x / sample_interval + 1][y / sample_interval + 1][z / sample_interval];
    let v111 = samples[x / sample_interval + 1][y / sample_interval + 1][z / sample_interval + 1];
    trilinear_interpolation(point, v000, v001, v010, v011, v100, v101, v110, v111)
}

pub fn trilinear_interpolation(
    point: Vec3,
    v000: f32,
    v001: f32,
    v010: f32,
    v011: f32,
    v100: f32,
    v101: f32,
    v110: f32,
    v111: f32,
) -> f32 {
    let c00 = v000 * (1.0 - point.x) + v100 * point.x;
    let c01 = v001 * (1.0 - point.x) + v101 * point.x;
    let c10 = v010 * (1.0 - point.x) + v110 * point.x;
    let c11 = v011 * (1.0 - point.x) + v111 * point.x;
    let c0 = c00 * (1.0 - point.y) + c10 * point.y;
    let c1 = c01 * (1.0 - point.y) + c11 * point.y;
    c0 * (1.0 - point.z) + c1 * point.z
}

//if v has maximum element m, returns the vector with m set to sign(m) and all other elements 0.
pub fn max_component_norm(v: Vec3) -> Vec3 {
    let abs = v.abs();
    if abs.x > abs.y && abs.x > abs.z {
        Vec3::new(v.x.signum(), 0.0, 0.0)
    } else if abs.y > abs.z {
        Vec3::new(0.0, v.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, v.z.signum())
    }
}

//if v has min element m, returns the vector with m set to sign(m) and all other elements 0.
pub fn min_component_norm(v: Vec3) -> Vec3 {
    let abs = v.abs();
    if abs.x < abs.y && abs.x < abs.z {
        Vec3::new(v.x.signum(), 0.0, 0.0)
    } else if abs.y < abs.z {
        Vec3::new(0.0, v.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, v.z.signum())
    }
}

//returns index of maximum element
pub fn max_index(v: Vec3) -> usize {
    if v.x > v.y && v.x > v.z {
        0
    } else if v.y > v.z {
        1
    } else {
        2
    }
}

//returns index of minimum element
pub fn min_index(v: Vec3) -> usize {
    if v.x < v.y && v.x < v.z {
        0
    } else if v.y < v.z {
        1
    } else {
        2
    }
}

//0s all other axes
pub fn pick_axis(v: Vec3, idx: usize) -> Vec3 {
    match idx {
        0 => Vec3::new(v.x, 0., 0.),
        1 => Vec3::new(0., v.y, 0.),
        2 => Vec3::new(0., 0., v.z),
        _ => panic!("index out of bounds"),
    }
}

//last method on https://mathworld.wolfram.com/SpherePointPicking.html (Muller 1959, Marsaglia 1972).
pub fn sample_sphere_surface(rng: &mut impl Rng) -> Vec3 {
    Vec3::new(
        rng.sample(StandardNormal),
        rng.sample(StandardNormal),
        rng.sample(StandardNormal),
    )
    .try_normalize()
    //should almost never fail, but provide a point in S^2 just in case
    .unwrap_or(Vec3::new(0.0, 1.0, 0.0))
}

//use in lerp(x,b,t) where x is current position, b is target dest
//lerps are exponential functions, so we have to correct the t
//speed is proportion that we should travel in 1 second
//TODO: https://chicounity3d.wordpress.com/2014/05/23/how-to-lerp-like-a-pro/
pub fn lerp_delta_time(speed: f32, dt: f32) -> f32 {
    //0.5 is arbitrary
    1.0 - ((1.0 - speed).powf(dt))
}

//https://easings.net/#easeInBack
//windup -> hit. good for punches!
pub fn ease_in_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    return c3 * t * t * t - c1 * t * t;
}

//https://easings.net/#easeInOutQuad
//used for the return after
pub fn ease_in_out_quad(t: f32) -> f32 {
    return if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    };
}

//this is used to make a continuous distribution discrete
//we find the smallest index of buckets greater than a given value
//buckets should be sorted in increasing order
pub struct Buckets<T> {
    pub buckets: Vec<(f32, T)>,
}

impl<T> Buckets<T> {
    //returns the first bucket with value greater than x
    //if buckets is non-empty and x is larger than all elements in the array, the last bucket is returned
    pub fn map(&self, x: f32) -> Option<&T> {
        self.buckets
            .iter()
            .find_or_last(|(b, _)| *b > x)
            .map(|(_, v)| v)
    }
    pub fn new(buckets: Vec<(f32, T)>) -> Self {
        Self { buckets }
    }
}

//for use in systems that need a local timer
//use in `Local<LocalRepeatingTimer<...>>`
#[derive(DerefMut, Deref)]
pub struct LocalRepeatingTimer<const INTERVAL_MS: u64>(pub Timer);

impl<const INTERVAL_MS: u64> Default for LocalRepeatingTimer<INTERVAL_MS> {
    fn default() -> Self {
        Self(Timer::new(
            Duration::from_millis(INTERVAL_MS),
            bevy::time::TimerMode::Repeating,
        ))
    }
}

pub trait ExtraOptions<T> {
    fn fallback(self, fallback: Option<T>) -> Option<T>;
}

impl<T> ExtraOptions<T> for Option<T> {
    //is self is none, returns fallback, otherwise, return self
    fn fallback(self, fallback: Option<T>) -> Option<T> {
        match self {
            x @ Some(_) => x,
            None => fallback,
        }
    }
}

pub struct SendEventCommand<T: bevy::prelude::Event>(pub T);

impl<T: bevy::prelude::Event> bevy::ecs::system::Command for SendEventCommand<T> {
    fn apply(self, world: &mut bevy::prelude::World) {
        world.send_event(self.0);
    }
}

pub fn get_wrapping<T>(slice: &[T], idx: usize) -> Option<&T> {
    match slice.len() {
        0 => None,
        len => slice.get(idx % len),
    }
}

//these can't be put in a trait.... great!
pub const fn f32_powi(b: f32, power: u32) -> f32 {
    let mut res = b;
    let mut idx = 0;
    //why can I not use for loops in const??
    while idx < power {
        res *= b;
        idx += 1;
    }
    res
}

pub const fn f64_powi(b: f64, power: u32) -> f64 {
    let mut res = b;
    let mut idx = 0;
    //why can I not use for loops in const??
    while idx < power {
        res *= b;
        idx += 1;
    }
    res
}

//assumes plane goes through (0,0,0)
pub fn project_onto_plane(vector: Vec3, plane_normal: Vec3) -> Vec3 {
    let dist = vector.dot(plane_normal);
    vector - dist * plane_normal
}

pub trait FlattenRef<'a, T> {
    fn flatten(self) -> Option<&'a T>;
}

impl<'a, T> FlattenRef<'a, T> for Option<&'a Option<T>> {
    fn flatten(self) -> Option<&'a T> {
        match self {
            Some(opt) => opt.as_ref(),
            None => None,
        }
    }
}
pub trait FlattenRefMut<'a, T> {
    fn flatten(self) -> Option<&'a mut T>;
}

impl<'a, T> FlattenRefMut<'a, T> for Option<&'a mut Option<T>> {
    fn flatten(self) -> Option<&'a mut T> {
        match self {
            Some(opt) => opt.as_mut(),
            None => None,
        }
    }
}
