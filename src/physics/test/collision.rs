#[allow(unused_imports)]
mod aabb {
    use crate::physics::collision::Aabb;
    use crate::BlockCoord;
    use bevy::prelude::*;
    use util::iterators::Volume;

    #[test]
    fn test_move_min() {
        let aabb = Aabb::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(-10.0, 19.0, 123.0));
        let delta = Vec3::new(10.0, -12.0, 15.0);
        let moved = aabb.move_min(delta);
        assert_eq!(aabb.min() + delta, moved.min());
        assert_eq!(aabb.max(), moved.max());
    }

    #[test]
    fn test_scale() {
        let aabb = Aabb::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(-10.0, 19.0, 123.0));
        let scale_factor = Vec3::new(10.0, 12.0, 15.0);
        let scaled_aabb = aabb.scale(scale_factor);
        //should keep center in place
        assert_eq!(aabb.center(), scaled_aabb.center());
        assert_eq!(aabb.size * scale_factor, scaled_aabb.size);
    }

    #[test]
    fn test_aabb_to_block_volume_inclusive() {
        let aabb = Aabb {
            size: Vec3::new(0.8, 1.5, 0.8),
            offset: Vec3::new(0.4, 0.75, 0.4),
        };
        let volume = aabb.to_volume(Vec3::new(-0.8, -0.8, -0.8));
        //since edges are on edges of cells, it should include those cells
        //aka: 2x3x2 = 12
        assert_eq!(volume.volume(), 12);
        assert_eq!(volume.max_corner, IVec3::new(1, 2, 1));
        assert_eq!(volume.min_corner, IVec3::new(-1, -1, -1));
    }

    #[test]
    fn test_aabb_to_block_volume() {
        let aabb = Aabb {
            size: Vec3::new(0.8, 1.5, 0.8),
            offset: Vec3::new(-0.4, -0.75, -0.4),
        };
        let volume = aabb.to_volume(Vec3::new(0.5, 0.8, 0.5));
        //should be fully enclosed in a 1x2x1 column
        assert_eq!(volume.volume(), 2);
        assert_eq!(volume.max_corner, IVec3::new(1, 2, 1));
        assert_eq!(volume.min_corner, IVec3::new(0, 0, 0));
    }
}
