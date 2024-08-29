use bevy::prelude::*;

use crate::lerp_delta_time;

pub struct UtilPlugin;

impl Plugin for UtilPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, smooth_look_to);
    }
}

#[derive(Component, Clone, Copy)]
pub struct SmoothLookTo {
    pub forward: Vec3,
    pub up: Vec3,
    pub speed: f32,
    pub enabled: bool,
}

impl SmoothLookTo {
    pub fn new(speed: f32) -> Self {
        Self { speed, ..default() }
    }
    pub fn with_forward(self, f: Vec3) -> Self {
        Self { forward: f, ..self }
    }
    pub fn with_up(self, up: Vec3) -> Self {
        Self { up, ..self }
    }
    pub fn with_enabled(self, enabled: bool) -> Self {
        Self { enabled, ..self }
    }
}

impl Default for SmoothLookTo {
    fn default() -> Self {
        Self {
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            speed: 0.5,
            enabled: false,
        }
    }
}

fn smooth_look_to(mut query: Query<(&mut Transform, &SmoothLookTo)>, time: Res<Time>) {
    const TOLERANCE: f32 = 0.01;
    for (mut tf, look) in query.iter_mut().filter(|(_, look)| look.enabled) {
        let rot = tf.looking_to(look.forward, look.up).rotation;
        tf.rotation = tf.rotation.slerp(
            rot,
            lerp_delta_time(look.speed, time.delta_seconds()).clamp(0.0, 1.0),
        );
        if tf.rotation.abs_diff_eq(rot, TOLERANCE) {
            tf.rotation = rot;
        }
    }
}
