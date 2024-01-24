use bevy::prelude::*;

use crate::{
    physics::{movement::Acceleration, PhysicsSystemSet},
    util::controls::PIController,
    world_utils::blockcast_checkers,
    BlockPhysics, Level,
};

use super::Float;

pub struct PIControllersPlugin;

impl Plugin for PIControllersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, update_floater.in_set(PhysicsSystemSet::Main));
    }
}

fn update_floater(
    mut query: Query<(
        &mut PIController<Float>,
        &mut Acceleration,
        &Float,
        &Transform,
    )>,
    physics_query: Query<&BlockPhysics>,
    time: Res<Time<Fixed>>,
    level: Res<Level>,
) {
    const CHECK_MULT: f32 = 10.0;
    for (mut controller, mut accel, float, tf) in query.iter_mut() {
        let target_ground_y = level.blockcast(
            tf.translation,
            Vec3::new(0.0, -float.target_ground_dist*CHECK_MULT, 0.0),
            |opt_block| blockcast_checkers::solid(&physics_query, opt_block),
        ).map(|hit| hit.hit_pos.y + float.target_ground_dist);
        let target_ceiling_y = level.blockcast(
            tf.translation,
            Vec3::new(0.0, float.target_ceiling_dist*CHECK_MULT, 0.0),
            |opt_block| blockcast_checkers::solid(&physics_query, opt_block),
        ).map(|hit| hit.hit_pos.y - float.target_ceiling_dist);
        let target_y = match (target_ground_y, target_ceiling_y) {
            (None, None) => {
                //not close enough to ground, cop out (but reset integral)
                // controller.reset();
                continue;
            }, 
            (None, Some(target_y)) => target_y,
            (Some(target_y), None) => target_y,
            (Some(target_ground), Some(target_ceiling)) => 0.5*(target_ground + target_ceiling), //take avg if we are in the middle
        };

        controller.target_value = target_y;
        let y_accel = controller.update(tf.translation.y, time.delta_seconds()).clamp(-float.max_force, float.max_force);
        accel.0.y += y_accel;
        info!("{:?}, target ground {:?}, target ceil {:?}, target_value {:?}", y_accel, target_ground_y, target_ceiling_y, target_y);
    }
}
