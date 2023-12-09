use bevy::math::Vec3;

use crate::physics::collision::Aabb;

#[cfg(test)]
mod collision;

#[test]
fn test_aabb_intersects_true() {
    assert!(Aabb::intersects(
        Aabb::new(Vec3::new(0.5, 1.0, 1.5)),
        Vec3::new(0.5, 2.0, 7.5),
        Aabb::new(Vec3::new(1.0, 2.0, 3.0))
    ))
}

#[test]
fn test_aabb_intersects_false() {
    assert!(!Aabb::intersects(
        Aabb::new(Vec3::new(0.5, 1.0, 1.5)),
        Vec3::new(2.5, 5.0, 7.5),
        Aabb::new(Vec3::new(1.0, 2.0, 3.0))
    ))
}

#[test]
fn test_aabb_distance_exterior() {
    assert_eq!(
        Aabb::axis_distance(
            Aabb::new(Vec3::new(0.5, 1.0, 1.5)),
            Vec3::new(2.5, 5.0, 7.5),
            Aabb::new(Vec3::new(1.0, 2.0, 3.0))
        ),
        Vec3::new(1.0, 2.0, 3.0)
    );
}

#[test]
fn test_aabb_distance_interior() {
    assert_eq!(
        Aabb::axis_distance(
            Aabb::new(Vec3::new(0.5, 1.0, 1.5)),
            Vec3::new(0.5, 2.0, 7.5),
            Aabb::new(Vec3::new(1.0, 2.0, 3.0))
        ),
        Vec3::new(-1.0, -1.0, 3.0)
    );
}
