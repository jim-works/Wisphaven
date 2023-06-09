use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    actors::*,
    physics::JUMPABLE_GROUP,
    world::{BlockCoord, Level},
};

use super::{Action, FrameMovement};

#[derive(Component, Default)]
pub struct RotateWithMouse {
    pub pitch: f32,
    pub roll: f32,
    pub yaw: f32,
    pub pitch_bound: f32,
    pub lock_pitch: bool,
    pub lock_yaw: bool,
}

#[derive(Component)]
pub struct FollowPlayer {}

#[derive(Component)]
pub struct PlayerActionOrigin {}

pub fn move_player(mut query: Query<(&ActionState<Action>, &mut FrameMovement), With<Player>>) {
    for (act, mut fm) in query.iter_mut() {
        let mut dv = Vec3::ZERO;
        dv.z -= if act.pressed(Action::MoveForward) {
            1.0
        } else {
            0.0
        };
        dv.z += if act.pressed(Action::MoveBack) {
            1.0
        } else {
            0.0
        };
        dv.x += if act.pressed(Action::MoveRight) {
            1.0
        } else {
            0.0
        };
        dv.x -= if act.pressed(Action::MoveLeft) {
            1.0
        } else {
            0.0
        };
        fm.0 = dv;
    }
}

//TODO: extract most of this into another system to look like move_player and do_planar_movement
pub fn jump_player(
    mut query: Query<
        (
            Entity,
            &mut Jump,
            &ActionState<Action>,
            &Collider,
            &GlobalTransform,
            &mut ExternalImpulse,
        ),
        With<Player>,
    >,
    ctx: Res<RapierContext>,
) {
    const DETECT_DIST: f32 = 0.05;
    for (entity, mut jump, act, collider, tf, mut ext_impulse) in query.iter_mut() {
        //check on ground
        let groups = QueryFilter {
            groups: Some(CollisionGroups::new(
                Group::ALL,
                Group::from_bits_truncate(JUMPABLE_GROUP),
            )),
            ..default()
        }
        .exclude_collider(entity);
        if let Some((_, _)) = ctx.cast_shape(
            tf.translation(),
            Quat::IDENTITY,
            Vec3::new(0.0, DETECT_DIST, 0.0),
            collider,
            1.0,
            groups,
        ) {
            jump.extra_jumps_remaining = jump.extra_jump_count;
            //don't use an extra jump since we are on the ground
            if act.just_pressed(Action::Jump) {
                ext_impulse.impulse.y += jump.current_height;
            }
        } else if jump.extra_jumps_remaining > 0 && act.just_pressed(Action::Jump) {
            //we aren't on the ground, so use an extra jump
            jump.extra_jumps_remaining -= 1;
            ext_impulse.impulse.y += jump.current_height;
        }
    }
}

pub fn rotate_mouse(
    mut query: Query<(&mut Transform, &mut RotateWithMouse, &ActionState<Action>)>,
) {
    const SENSITIVITY: f32 = 0.01;
    for (mut tf, mut rotation, action) in query.iter_mut() {
        if let Some(delta) = action.axis_pair(Action::Look) {
            if !rotation.lock_yaw {
                rotation.yaw -= delta.x() * SENSITIVITY;
            }
            if !rotation.lock_pitch {
                rotation.pitch -= delta.y() * SENSITIVITY;
            }

            rotation.pitch = rotation
                .pitch
                .clamp(-rotation.pitch_bound, rotation.pitch_bound);

            tf.rotation = Quat::from_axis_angle(Vec3::Y, rotation.yaw)
                * Quat::from_axis_angle(Vec3::X, rotation.pitch)
                * Quat::from_axis_angle(Vec3::Z, rotation.roll);
        }
    }
}

pub fn follow_local_player(
    player_query: Query<(&Transform, &RotateWithMouse), With<LocalPlayer>>,
    mut follow_query: Query<
        (&mut Transform, Option<&mut RotateWithMouse>),
        (With<FollowPlayer>, Without<LocalPlayer>),
    >,
) {
    if let Ok((player_tf, player_rot)) = player_query.get_single() {
        for (mut follow_tf, opt_follow_rot) in follow_query.iter_mut() {
            follow_tf.translation = player_tf.translation + Vec3::new(0.0, 1.5, 0.0);
            if let Some(mut follow_rot) = opt_follow_rot {
                follow_rot.yaw = player_rot.yaw;
            }
        }
    }
}

//todo: mesh neighbors (add batch set block in level that takes in commands to do this)
pub fn player_punch(
    mut commands: Commands,
    camera_query: Query<
        (&Transform, &ActionState<Action>),
        (
            With<PlayerActionOrigin>,
            With<FollowPlayer>,
            Without<LocalPlayer>,
        ),
    >,
    mut level: ResMut<Level>,
) {
    if let Ok((tf, act)) = camera_query.get_single() {
        if act.just_pressed(Action::Punch) {
            if let Some(hit) = level.blockcast(tf.translation, tf.forward() * 10.0) {
                level.set_block(hit.block_pos, crate::world::BlockType::Empty, &mut commands);
            }
        }
    }
}

//todo: mesh neighbors (add batch set block in level that takes in commands to do this)
pub fn player_use(
    mut commands: Commands,
    camera_query: Query<
        (&Transform, &ActionState<Action>),
        (
            With<PlayerActionOrigin>,
            With<FollowPlayer>,
            Without<LocalPlayer>,
        ),
    >,
    mut level: ResMut<Level>,
) {
    const SIZE: i32 = 10;
    if let Ok((tf, act)) = camera_query.get_single() {
        if act.just_pressed(Action::Use) {
            // if let Some(hit) = level.blockcast(tf.translation, tf.forward()*10.0) {
            //     level.set_block(hit.block_pos+hit.normal, crate::world::BlockType::Basic(0), &mut commands);
            // }
            if let Some(hit) = level.blockcast(tf.translation, tf.forward() * 200.0) {
                let mut changes = Vec::with_capacity((SIZE*SIZE*SIZE) as usize);
                for x in -SIZE..SIZE {
                    for y in -SIZE..SIZE {
                        for z in -SIZE..SIZE {
                            changes.push((
                                hit.block_pos + BlockCoord::new(x, y, z),
                                crate::world::BlockType::Empty,
                            ));
                        }
                    }
                }
                level.batch_set_block(changes.into_iter(), &mut commands);
            }
        }
    }
}
