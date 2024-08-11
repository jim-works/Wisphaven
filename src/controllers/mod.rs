pub mod player_controller;
use leafwing_input_manager::prelude::*;
use player_controller::*;

mod input;
pub use input::*;
mod pi_controllers;

use bevy::prelude::*;

use crate::{
    actors::{abilities::dash::CurrentlyDashing, Jump, MoveSpeed},
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
                jump_player,
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
#[derive(Component)]
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
    pub frame_jump: FrameJump,
    pub move_speed: MoveSpeed,
    pub jump: Jump,
    pub mode: MovementMode,
}

impl Default for ControllableBundle {
    fn default() -> Self {
        ControllableBundle {
            frame_movement: TickMovement(Vec3::default()),
            move_speed: MoveSpeed::default(),
            jump: Jump::default(),
            frame_jump: FrameJump(false),
            mode: MovementMode::default(),
        }
    }
}

fn do_tick_movement(
    mut query: Query<(
        &TickMovement,
        &mut crate::physics::movement::Velocity,
        &MoveSpeed,
        &MovementMode,
        Option<&CollidingDirections>,
    ), Without<CurrentlyDashing>>,
) {
    const EPSILON: f32 = 1e-3;
    for (fm, mut v, ms, mode, opt_grounded) in query.iter_mut() {
        let speed = fm.0.length();
        let has_input = speed > EPSILON;
        let acceleration = ms.get_accel(
            opt_grounded.is_some_and(|x| x.0.contains(DirectionFlags::NegY)),
            has_input,
        );
        //don't actively resist sliding if v is small to reduce jitter
        if v.0.length_squared() < EPSILON * EPSILON {
            continue;
        }

        //global space
        let mut v_desired = if speed > 1.0 {
            fm.0 * (ms.max_speed / speed)
        } else {
            fm.0 * ms.max_speed
        };
        if *mode != MovementMode::Flying {
            v_desired.y = 0.0;
        }

        //create impulse that would set us to the desired speed
        //this impulse will be pushing back into the circle of radius ms.max
        let mut dv = v_desired - v.0;
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
