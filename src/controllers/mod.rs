pub mod player_controller;
use leafwing_input_manager::prelude::*;
use player_controller::*;

mod input;
pub use input::*;
mod pi_controllers;

use bevy::prelude::*;

use crate::{
    actors::{abilities::dash::CurrentlyDashing, ghost::FloatBoost, Jump, MoveSpeed},
    physics::{collision::CollidingDirections, PhysicsSystemSet},
    util::DirectionFlags,
    world::LevelSystemSet,
};

pub struct ControllersPlugin;

impl Plugin for ControllersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            InputManagerPlugin::<Action>::default(),
            pi_controllers::PIControllersPlugin,
        ))
        //player
        .add_systems(
            Update,
            (
                rotate_mouse,
                boost_float_player,
                move_player,
                dash_player,
                follow_local_player,
                player_punch,
                player_use,
                player_scroll_inventory,
                toggle_player_flight,
            )
                .in_set(LevelSystemSet::Main),
        )
        //common
        .add_systems(
            FixedUpdate,
            (do_jump, do_tick_movement).in_set(PhysicsSystemSet::Main),
        );
    }
}

//vector is the desired proportion of the movespeed to use, it's not normalized, but if the magnitude is greater than 1 it will be.
//global space
#[derive(Component)]
pub struct TickMovement(pub Vec3);

//reset every frame in do_jump
#[derive(Component, Default)]
pub struct FrameJump(pub bool);

//reset every frame in do_dash
#[derive(Component)]
pub struct FrameDash(pub Vec3);

#[derive(Component, Default, Eq, PartialEq, Clone, Copy, Debug)]
pub enum MovementMode {
    #[default]
    Normal,
    Flying,
}

//should have a PhysicsObjectBundle too
#[derive(Bundle)]
pub struct ControllableBundle {
    pub frame_movement: TickMovement,
    pub move_speed: MoveSpeed,
    pub mode: MovementMode,
}

impl Default for ControllableBundle {
    fn default() -> Self {
        ControllableBundle {
            frame_movement: TickMovement(Vec3::default()),
            move_speed: MoveSpeed::default(),
            mode: MovementMode::default(),
        }
    }
}

#[derive(Bundle, Default)]
pub struct JumpBundle {
    pub jump: Jump,
    pub frame_jump: FrameJump,
}

fn do_tick_movement(
    mut query: Query<
        (
            &TickMovement,
            &mut crate::physics::movement::Velocity,
            &MoveSpeed,
            &MovementMode,
            Option<&CollidingDirections>,
            Option<&FloatBoost>,
        ),
        Without<CurrentlyDashing>,
    >,
) {
    const EPSILON: f32 = 1e-3;
    const HIGH_SPEED_MODE_MULT: f32 = 1.5;
    for (fm, mut v, ms, mode, opt_grounded, opt_boost) in query.iter_mut() {
        let input_speed = fm.0.length();

        let current_velocity = if *mode != MovementMode::Flying {
            Vec3::new(v.0.x, 0.0, v.0.z)
        } else {
            v.0
        };

        let current_speed = current_velocity.length();
        let norm_velocity = current_velocity / current_speed;
        let has_input = input_speed > EPSILON;

        //global space
        let mut v_desired = if input_speed > 1.0 {
            fm.0 * (ms.max_speed / input_speed)
        } else {
            fm.0 * ms.max_speed
        };
        if *mode != MovementMode::Flying {
            v_desired.y = 0.0;
        }

        let acceleration = ms.get_accel(
            opt_grounded.is_some_and(|x| x.0.contains(DirectionFlags::NegY)),
            has_input,
            fm.0 / input_speed,
            norm_velocity,
        );

        //don't actively resist sliding if v is small to reduce jitter
        if current_speed < EPSILON {
            //v is small, so we can just aply the whole acceleration
            v.0 += acceleration * v_desired;
            continue;
        }

        let float_boost_active = opt_boost.is_some_and(|b| b.enabled);
        if current_speed > ms.max_speed * HIGH_SPEED_MODE_MULT || float_boost_active {
            //high speed mode - don't allow acceleration in the direction of velocity, but allow movement on the transverse axis
            let desired_speed = v_desired.length();
            if desired_speed < EPSILON {
                continue; //maintain momentum if not moving
            }
            let main_axis = v_desired.project_onto_normalized(norm_velocity);
            let orthogonal_axis = v_desired - main_axis;
            let mut delta = orthogonal_axis;
            //only allow movement on the main axis if it's against current velocity
            if v_desired.dot(norm_velocity) < 0. || current_speed < ms.max_speed {
                delta += main_axis;
            }
            v.0 += delta * acceleration;
        } else {
            //low speed mode - enables more precise movement and "auto braking" behavior
            //we will actively resist sliding
            //create impulse that would set us to the desired speed
            //this impulse will be pushing back into the circle of radius ms.max
            let mut dv = v_desired - current_velocity;
            if *mode != MovementMode::Flying {
                dv.y = 0.0;
            }

            let a_desired = dv * acceleration;
            let a_mag = a_desired.length();
            v.0 += if a_mag > acceleration {
                acceleration * a_desired / a_mag
            } else {
                a_desired
            };
        }
    }
}

fn do_jump(
    mut query: Query<(
        &mut FrameJump,
        &mut crate::physics::movement::Velocity,
        &mut Jump,
        &CollidingDirections,
    )>,
) {
    for (mut fj, mut v, mut jump, collisions) in query.iter_mut() {
        let grounded = collisions.0.contains(DirectionFlags::NegY);
        if grounded {
            jump.extra_jumps_remaining = jump.extra_jump_count;
        }
        if !fj.0 {
            continue;
        }
        if grounded {
            //on ground, don't use extra jump
            v.y += jump.current_height;
        } else if jump.extra_jumps_remaining > 0 {
            //we aren't on the ground, so use an extra jump
            jump.extra_jumps_remaining -= 1;
            v.y += jump.current_height;
        }
        fj.0 = false; //reset frame jump
    }
}
