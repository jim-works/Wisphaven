use std::time::Duration;

use bevy::prelude::*;

use crate::{actors::MoveSpeed, physics::movement::Velocity, util::{ease_out_quad, inverse_lerp, lerp}, LevelSystemSet};

use super::StaminaCost;

pub struct DashPlugin;

impl Plugin for DashPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, do_dash.in_set(LevelSystemSet::Main));
    }
}

#[derive(Component, Clone, Copy)]
pub struct Dash {
    pub base_speed: f32,
    pub current_speed: f32,
    pub dash_duration: Duration,
    pub begin_fade_offset: Duration,
    pub stamina_cost: StaminaCost
}

impl Default for Dash {
    fn default() -> Self {
        Self {
            base_speed: 10.0,
            current_speed: 10.0,
            dash_duration: Duration::from_secs_f32(0.5),
            begin_fade_offset: Duration::from_secs_f32(0.25),
            stamina_cost: StaminaCost::new(1.0)
        }
    }
}

impl Dash {
    pub fn new(speed: f32) -> Self {
        Self {
            base_speed: speed,
            current_speed: speed,
            ..Default::default()
        }
    }
}

#[derive(Component)]
#[component(storage="SparseSet")]
pub struct CurrentlyDashing {
    fade_start_time: Duration,
    end_time: Duration,
    speed: f32,
    initial_v: Vec3,
}

impl CurrentlyDashing {
    pub fn new(dash: Dash, current_time: Duration, initial_v: Vec3) -> Self {
        Self {
            end_time: dash.dash_duration + current_time,
            fade_start_time: dash.begin_fade_offset + current_time,
            speed: dash.current_speed,
            initial_v
        }
    }
}

//when adding movement modes, be sure to update do_tick_movement
fn do_dash(
    mut commands: Commands,
    mut dashing_query: Query<(Entity, &GlobalTransform, &mut Velocity, &CurrentlyDashing, Option<&MoveSpeed>)>,
    time: Res<Time>
) {
    let curr_time = time.elapsed();
    for (entity, tf, mut v, dash, ms_opt) in dashing_query.iter_mut() {
        if curr_time >= dash.end_time {
            if let Some(mut ec) = commands.get_entity(entity) {
                ec.remove::<CurrentlyDashing>();
            }
            //fade to max move speed, initial speed, or dash speed (if max ms is larger)
            let initial_speed = Vec3::new(dash.initial_v.x,0.,dash.initial_v.z).length();
            let ms = ms_opt.map(|ms| ms.max_speed).unwrap_or_default().min(dash.speed);
            v.0 = tf.forward()*ms.max(initial_speed);
        }
        else if curr_time >= dash.fade_start_time {
            let fade_amount = inverse_lerp(dash.fade_start_time.as_secs_f32(), dash.end_time.as_secs_f32(), curr_time.as_secs_f32());
            //fade to max move speed, initial speed, or dash speed (if max ms is larger)
            let initial_speed = Vec3::new(dash.initial_v.x,0.,dash.initial_v.z).length();
            let ms = ms_opt.map(|ms| ms.max_speed).unwrap_or_default().min(dash.speed);
            let speed = lerp(dash.speed, ms.max(initial_speed), ease_out_quad(fade_amount));
            v.0 = tf.forward()*speed;
        } else {
            v.0 = tf.forward()*dash.speed;
        }
    }
}