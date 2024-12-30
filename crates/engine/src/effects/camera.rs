use bevy::prelude::*;

use crate::{
    actors::MoveSpeed,
    physics::movement::Velocity,
    util::{inverse_lerp, lerp, lerp_delta_time, DEG_TO_RAD},
};

pub struct CameraEffectsPlugin;

impl Plugin for CameraEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, do_camera_fov_effect);
    }
}

#[derive(Bundle, Default)]
pub struct CameraEffectsBundle {
    fov_effect: CameraFOVEffect,
}

#[derive(Component)]
pub struct CameraFOVEffect {
    pub extra_fov_rad: f32,
    pub min_ms_mult: f32,
    pub max_ms_mult: f32,
    pub change_speed: f32,
    fov_added: f32,
}

impl Default for CameraFOVEffect {
    fn default() -> Self {
        Self {
            extra_fov_rad: 30.0*DEG_TO_RAD,
            min_ms_mult: 1.5,
            max_ms_mult: 20.0,
            change_speed: 0.9,
            fov_added: 0.0,
        }
    }
}

fn do_camera_fov_effect(
    mut query: Query<(&mut Projection, &mut CameraFOVEffect, &Velocity, &MoveSpeed)>,
    time: Res<Time>,
) {
    for (mut projection_type, mut effect, v, ms) in query.iter_mut() {
        let Projection::Perspective(ref mut projection) = projection_type.as_mut() else {
            continue;
        };
        let effect_progress = inverse_lerp(
            effect.min_ms_mult,
            effect.max_ms_mult,
            if ms.max_speed > 0.0 {
                v.0.length() / ms.max_speed
            } else {
                0.0
            },
        )
        .clamp(0.0, 1.0);
        let target_fov_add = lerp(0.0, effect.extra_fov_rad, effect_progress);
        let new_fov_add = lerp(
            effect.fov_added,
            target_fov_add,
            lerp_delta_time(effect.change_speed, time.delta_secs()),
        );
        let fov_delta = new_fov_add - effect.fov_added;
        effect.fov_added = new_fov_add;
        projection.fov += fov_delta;
    }
}
