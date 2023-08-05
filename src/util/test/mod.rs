use crate::util::max_component_norm;


use super::*;

#[test]
pub fn test_max_component_norm() {
    //test in each direction
    assert_eq!(Vec3::new(1.0,0.0,0.0), max_component_norm(Vec3::new(0.7,-0.5,0.5)));
    assert_eq!(Vec3::new(-1.0,0.0,0.0), max_component_norm(Vec3::new(-0.7,0.5,0.5)));
    assert_eq!(Vec3::new(0.0,1.0,0.0), max_component_norm(Vec3::new(0.5,0.7,0.5)));
    assert_eq!(Vec3::new(0.0,-1.0,0.0), max_component_norm(Vec3::new(-0.5,-0.7,0.5)));
    assert_eq!(Vec3::new(0.0,0.0,1.0), max_component_norm(Vec3::new(0.5,-0.5,0.7)));
    assert_eq!(Vec3::new(0.0,0.0,-1.0), max_component_norm(Vec3::new(-0.5,0.5,-0.7)));
}

#[test]
pub fn vec3_to_direction() {
    //test in each direction
    assert_eq!(Direction::PosX, Direction::from(Vec3::new(0.7,-0.5,0.5)));
    assert_eq!(Direction::NegX, Direction::from(Vec3::new(-0.7,0.5,0.5)));
    assert_eq!(Direction::PosY, Direction::from(Vec3::new(0.5,0.7,0.5)));
    assert_eq!(Direction::NegY, Direction::from(Vec3::new(-0.5,-0.7,0.5)));
    assert_eq!(Direction::PosZ, Direction::from(Vec3::new(0.5,-0.5,0.7)));
    assert_eq!(Direction::NegZ, Direction::from(Vec3::new(-0.5,0.5,-0.7)));
}