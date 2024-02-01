mod aabb {
    #[allow(unused_imports)]
    use bevy::prelude::*;
    #[allow(unused_imports)]
    use crate::physics::collision::Aabb;

    #[test]
    fn test_move_min() {
        let aabb = Aabb::new(Vec3::new(1.0,2.0,3.0), Vec3::new(-10.0,19.0,123.0));
        let delta = Vec3::new(10.0,-12.0,15.0);
        let moved = aabb.move_min(delta);
        assert_eq!(aabb.min()+delta, moved.min());
        assert_eq!(aabb.max(), moved.max());
    }

    #[test]
    fn test_scale() {
        let aabb = Aabb::new(Vec3::new(1.0,2.0,3.0), Vec3::new(-10.0,19.0,123.0));
        let scale_factor = Vec3::new(10.0,12.0,15.0);
        let scaled_aabb = aabb.scale(scale_factor);
        //should keep center in place
        assert_eq!(aabb.center(), scaled_aabb.center());
        assert_eq!(aabb.size*scale_factor, scaled_aabb.size);
    }
}