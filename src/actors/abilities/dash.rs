use std::time::Duration;

use bevy::prelude::*;

use crate::{physics::movement::Velocity, LevelSystemSet};

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
    pub stamina_cost: StaminaCost
}

impl Default for Dash {
    fn default() -> Self {
        Self {
            base_speed: 10.0,
            current_speed: 10.0,
            dash_duration: Duration::from_secs_f32(0.25),
            stamina_cost: StaminaCost::new(5.0)
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
    end_time: Duration,
    speed: f32,
    extra_v: Vec3,
}

impl CurrentlyDashing {
    pub fn new(dash: Dash, current_time: Duration) -> Self {
        Self {
            end_time: dash.dash_duration + current_time,
            speed: dash.current_speed,
            extra_v: Vec3::ZERO
        }
    }
}

fn do_dash(
    mut commands: Commands,
    mut dashing_query: Query<(Entity, &GlobalTransform, &mut Velocity, &mut CurrentlyDashing)>,
    time: Res<Time>
) {
    let curr_time = time.elapsed();
    for (entity, tf, mut v, mut dash) in dashing_query.iter_mut() {
        v.0 -= dash.extra_v;
        if curr_time >= dash.end_time {
            if let Some(mut ec) = commands.get_entity(entity) {
                ec.remove::<CurrentlyDashing>();
            }
            continue;
        }
        dash.extra_v = tf.forward()*dash.speed;
        v.0 += dash.extra_v;
        info!("used");
    }
}