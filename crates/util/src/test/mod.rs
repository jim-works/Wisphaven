use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{max_component_norm, DEG_TO_RAD, RAD_TO_DEG};

use super::direction::Direction;

#[cfg(test)]
mod iterators;
#[cfg(test)]
mod string;

#[test]
fn test_max_component_norm() {
    //test in each direction
    assert_eq!(
        Vec3::new(1.0, 0.0, 0.0),
        max_component_norm(Vec3::new(0.7, -0.5, 0.5))
    );
    assert_eq!(
        Vec3::new(-1.0, 0.0, 0.0),
        max_component_norm(Vec3::new(-0.7, 0.5, 0.5))
    );
    assert_eq!(
        Vec3::new(0.0, 1.0, 0.0),
        max_component_norm(Vec3::new(0.5, 0.7, 0.5))
    );
    assert_eq!(
        Vec3::new(0.0, -1.0, 0.0),
        max_component_norm(Vec3::new(-0.5, -0.7, 0.5))
    );
    assert_eq!(
        Vec3::new(0.0, 0.0, 1.0),
        max_component_norm(Vec3::new(0.5, -0.5, 0.7))
    );
    assert_eq!(
        Vec3::new(0.0, 0.0, -1.0),
        max_component_norm(Vec3::new(-0.5, 0.5, -0.7))
    );
}

#[test]
fn vec3_to_direction() {
    //test in each direction
    assert_eq!(Direction::PosX, Direction::from(Vec3::new(0.7, -0.5, 0.5)));
    assert_eq!(Direction::NegX, Direction::from(Vec3::new(-0.7, 0.5, 0.5)));
    assert_eq!(Direction::PosY, Direction::from(Vec3::new(0.5, 0.7, 0.5)));
    assert_eq!(Direction::NegY, Direction::from(Vec3::new(-0.5, -0.7, 0.5)));
    assert_eq!(Direction::PosZ, Direction::from(Vec3::new(0.5, -0.5, 0.7)));
    assert_eq!(Direction::NegZ, Direction::from(Vec3::new(-0.5, 0.5, -0.7)));
}

#[test]
fn test_deg_to_rad() {
    assert!((90.0 * DEG_TO_RAD - PI / 2.0).abs() < 0.00001);
    assert!((180.0 * DEG_TO_RAD - PI).abs() < 0.00001);
}

#[test]
fn test_rad_to_deg() {
    assert!((90.0 - RAD_TO_DEG * PI / 2.0).abs() < 0.00001);
    assert!((180.0 - RAD_TO_DEG * PI).abs() < 0.00001);
}
