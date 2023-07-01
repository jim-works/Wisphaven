mod direction;
pub use direction::*;

mod spline;
pub use spline::*;

mod noise;
pub use noise::*;

pub mod l_system;

mod numerical_traits;
pub use numerical_traits::*;

pub mod plugin;

use bevy::prelude::Vec3;

use rand::prelude::*;
use rand_distr::StandardNormal;

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
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
    1.0-((1.0-speed).powf(dt))
}