mod player_controller;
use leafwing_input_manager::prelude::*;
pub use player_controller::*;

mod input;
pub use input::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    actors::{Jump, MoveSpeed},
    physics::JUMPABLE_GROUP,
    world::LevelSystemSet,
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
                )
                    .in_set(LevelSystemSet::Main),
            )
            //common
            .add_systems(
                PostUpdate,
                (update_grounded, do_jump.after(update_grounded), do_planar_movement.after(update_grounded)).in_set(LevelSystemSet::PostUpdate),
            );
    }
}

//vector is the desired proportion of the movespeed to use, it's not normalized, but if the magnitude is greater than 1 it will be.
//reset every frame in do_planar_movement
#[derive(Component)]
pub struct FrameMovement(pub Vec3);

//reset every frame in do_jump
#[derive(Component)]
pub struct FrameJump(pub bool);

//updated every frame in update_grounded
//affects movement speed
#[derive(Component)]
pub struct Grounded(pub bool);

//should have a PhysicsObjectBundle too
#[derive(Bundle)]
pub struct ControllableBundle {
    pub frame_movement: FrameMovement,
    pub frame_jump: FrameJump,
    pub move_speed: MoveSpeed,
    pub jump: Jump,
    pub grounded: Grounded,
}

impl Default for ControllableBundle {
    fn default() -> Self {
        ControllableBundle {
            frame_movement: FrameMovement(Vec3::default()),
            move_speed: MoveSpeed::default(),
            jump: Jump::default(),
            frame_jump: FrameJump(false),
            grounded: Grounded(false),
        }
    }
}

fn do_planar_movement(
    mut query: Query<(
        &mut FrameMovement,
        &mut ExternalImpulse,
        &Transform,
        &Velocity,
        &MoveSpeed,
        Option<&Grounded>,
    )>,
    time: Res<Time>,
) {
    const EPSILON: f32 = 1e-3;
    for (mut fm, mut impulse, tf, v, ms, opt_grounded) in query.iter_mut() {
        let local_movement = fm.0;
        let local_speed = local_movement.length();
        let acceleration = ms.get_accel(opt_grounded.is_some_and(|x| x.0));
        //don't actively resist sliding if no input is provided (also smooths out jittering)
        if local_speed < EPSILON {
            fm.0 = Vec3::ZERO;
            continue;
        }
        //global space
        let mut v_desired = if local_speed > 1.0 {
            tf.rotation * (local_movement * (ms.max_speed / local_speed))
        } else {
            tf.rotation * local_movement * ms.max_speed
        };
        v_desired.y = 0.0;

        //create impulse that pushes us in the desired direction
        //this impulse will be pushing back into the circle of radius ms.max, so no need to normalize
        let mut dv = v_desired - v.linvel;
        dv.y = 0.0;
        let dv_len = dv.length();
        //don't overcorrect
        if dv_len > EPSILON {
            impulse.impulse += dv * (acceleration * time.delta_seconds() / dv_len);
        }
        fm.0 = Vec3::ZERO;
    }
}

fn update_grounded(
    mut query: Query<(Entity, &mut Grounded, &GlobalTransform, &Collider)>,
    ctx: Res<RapierContext>,
) {
    const EPSILON: f32 = 1e-3;
    const DETECT_DIST: f32 = 0.05;
    for (entity, mut grounded, tf, col) in query.iter_mut() {
        //check on ground
        let groups = QueryFilter {
            groups: Some(CollisionGroups::new(
                Group::ALL,
                Group::from_bits_truncate(JUMPABLE_GROUP),
            )),
            ..default()
        }
        .exclude_collider(entity);
        grounded.0 = ctx
            .cast_shape(
                tf.translation(),
                Quat::IDENTITY,
                Vec3::new(0.0, DETECT_DIST, 0.0),
                col,
                1.0,
                groups,
            )
            .is_some();
    }
}

fn do_jump(
    mut query: Query<(
        &mut FrameJump,
        &mut ExternalImpulse,
        &mut Jump,
        &Grounded,
    )>
) {
    for (mut fj, mut impulse, mut jump, grounded) in query.iter_mut() {
        if !fj.0 {
            continue;
        }
        if grounded.0 {
            //on ground, refill jumps
            jump.extra_jumps_remaining = jump.extra_jump_count;
            impulse.impulse.y += jump.current_height;
        } else if jump.extra_jumps_remaining > 0 {
            //we aren't on the ground, so use an extra jump
            jump.extra_jumps_remaining -= 1;
            impulse.impulse.y += jump.current_height;
        }
        fj.0 = false; //reset frame jump
    }
}
