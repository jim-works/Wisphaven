use bevy::prelude::*;

use crate::util::lerp_delta_time;

pub struct UtilPlugin;

impl Plugin for UtilPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(smooth_look_to);
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SmoothLookTo {
    pub to: Vec3,
    pub up: Vec3,
    pub speed: f32
}

fn smooth_look_to (
    mut query: Query<(Entity, &mut Transform, &SmoothLookTo)>,
    mut commands: Commands,
    time: Res<Time>
) {
    const TOLERANCE: f32 = 0.01;
    for (entity, mut tf, look) in query.iter_mut() {
        let rot = tf.looking_to(look.to, look.up).rotation;
        tf.rotation = tf.rotation.slerp(rot, lerp_delta_time(look.speed,time.delta_seconds()));
        if tf.rotation.abs_diff_eq(rot, TOLERANCE) {
            tf.rotation = rot;
            if let Some(mut ec) = commands.get_entity(entity) {
                ec.remove::<SmoothLookTo>();
            }
        }
    }
}
