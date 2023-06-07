mod direction;
pub use direction::*;

mod spline;
pub use spline::*;

mod noise;
pub use noise::*;

use bevy::prelude::Vec3;

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a*(1.0-t)+b*t
}

//if v has maximum element m, returns the vector with m set to sign(m) and all other elements 0.
pub fn max_component_norm(v: Vec3) -> Vec3 {
    let abs = v.abs();
    if abs.x > abs.y && abs.x > abs.z {
        return Vec3::new(v.x.signum(),0.0,0.0)
    } else if abs.y > abs.z {
        return Vec3::new(0.0,v.y.signum(),0.0)
    } else {
        return Vec3::new(0.0,0.0,v.z.signum())
    }
}