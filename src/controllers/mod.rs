mod player_controller;
use leafwing_input_manager::prelude::*;
pub use player_controller::*;

mod input;
pub use input::*;

use bevy::prelude::*;

use crate::{
    actors::{Jump, MoveSpeed},
    physics::collision::CollidingDirections,
    world::LevelSystemSet, util::DirectionFlags,
};

pub struct ControllersPlugin;

impl Plugin for ControllersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<Action>::default())
            //player
            .add_systems(
                Update,
                (
                    rotate_mouse,
                    jump_player,
                    move_player,
                    follow_local_player,
                    player_punch,
                    player_use,
                    player_scroll_inventory,
                    toggle_player_flight
                )
                    .in_set(LevelSystemSet::Main),
            )
            //common
            .add_systems(
                PostUpdate,
                (do_jump, do_frame_movement).in_set(LevelSystemSet::PostUpdate),
            );
    }
}

//vector is the desired proportion of the movespeed to use, it's not normalized, but if the magnitude is greater than 1 it will be.
//global space
//reset every frame in do_planar_movement
#[derive(Component)]
pub struct FrameMovement(pub Vec3);

//reset every frame in do_jump
#[derive(Component)]
pub struct FrameJump(pub bool);

#[derive(Component, Default, Eq, PartialEq, Clone, Copy, Debug)]
pub enum MovementMode {
    #[default]
    Normal,
    Flying,
}

//should have a PhysicsObjectBundle too
#[derive(Bundle)]
pub struct ControllableBundle {
    pub frame_movement: FrameMovement,
    pub frame_jump: FrameJump,
    pub move_speed: MoveSpeed,
    pub jump: Jump,
    pub mode: MovementMode
}

impl Default for ControllableBundle {
    fn default() -> Self {
        ControllableBundle {
            frame_movement: FrameMovement(Vec3::default()),
            move_speed: MoveSpeed::default(),
            jump: Jump::default(),
            frame_jump: FrameJump(false),
            mode: MovementMode::default(),
        }
    }
}

fn do_frame_movement(
    mut query: Query<(
        &mut FrameMovement,
        &crate::physics::movement::Velocity,
        &mut crate::physics::movement::Acceleration,
        &MoveSpeed,
        &MovementMode,
        Option<&CollidingDirections>,
    )>,
    time: Res<Time>,
) {
    const EPSILON: f32 = 1e-3;
    for (mut fm, v, mut a, ms, mode, opt_grounded) in query.iter_mut() {
        let speed = fm.0.length();
        let acceleration = ms.get_accel(opt_grounded.is_some_and(|x| x.0.contains(DirectionFlags::NegY)));
        //don't actively resist sliding if no input is provided (also smooths out jittering)
        if speed < EPSILON {
            fm.0 = Vec3::ZERO;
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

        //create impulse that pushes us in the desired direction
        //this impulse will be pushing back into the circle of radius ms.max, so no need to normalize
        let mut dv = v_desired - v.0;
        if *mode != MovementMode::Flying {
            dv.y = 0.0;
        }
        let dv_len = dv.length();
        //don't overcorrect
        if dv_len > EPSILON {
            a.0 += dv * (acceleration * time.delta_seconds() / dv_len);
        }
        fm.0 = Vec3::ZERO;
    }
}

fn do_jump(
    mut query: Query<(
        &mut FrameJump,
        &mut crate::physics::movement::Velocity,
        &mut Jump,
        &CollidingDirections,
    )>
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
