use std::time::Duration;

use bevy::prelude::*;
use big_brain::prelude::*;

use crate::{
    controllers::{FrameJump, FrameMovement},
    world::{BlockCoord, BlockPhysics, Level, LevelSystemSet},
};

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            walk_to_destination_action
                .in_set(BigBrainSet::Actions)
                .in_set(LevelSystemSet::Main),
        );
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct TargetDestination(pub Vec3);

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct WalkToDestinationAction {
    wait_timer: Timer,
}

impl Default for WalkToDestinationAction {
    fn default() -> Self {
        let mut wait_timer = Timer::from_seconds(0.0, TimerMode::Once);
        //tick the wait timer so it's finished by default
        wait_timer.tick(Duration::from_secs(1));
        Self {
            wait_timer
        }
    }
}

fn walk_to_destination_action(
    mut info: Query<(
        &GlobalTransform,
        &TargetDestination,
        &mut FrameMovement,
        Option<&mut FrameJump>,
    )>,
    mut query: Query<(&Actor, &mut ActionState, &mut WalkToDestinationAction)>,
    level: Res<Level>,
    block_physics: Query<&BlockPhysics>,
    time: Res<Time>,
) {
    const EPSILON: f32 = 0.1;
    const SLOW_DIST: f32 = 0.5;
    const JUMP_DIST: f32 = 0.75;
    const JUMP_COOLDOWN: Duration = Duration::from_millis(500);
    for (Actor(actor), mut state, mut action) in query.iter_mut() {
        if let Ok((tf, TargetDestination(dest), mut fm, fj)) = info.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    *state = ActionState::Executing;
                }
                ActionState::Executing | ActionState::Cancelled => {
                    //we check the wait timer before moving.
                    //if we move into a wall right after jumping, the friction on the wall will make the jump go nowhere
                    action.wait_timer.tick(time.delta());
                    if !action.wait_timer.finished() {
                        continue;
                    }
                    let delta = Vec3::new(dest.x, 0.0, dest.z)
                        - Vec3::new(tf.translation().x, 0.0, tf.translation().z);
                    if delta.length_squared() < EPSILON * EPSILON {
                        //we are close enough
                        *state = ActionState::Success;
                        return;
                    }
                    fm.0 = if delta.length_squared() < SLOW_DIST * SLOW_DIST {
                        //lerp to walk slower to not overshoot target
                        delta / delta.length_squared()
                    } else {
                        delta.normalize()
                    };
                    //test if we need to jump over a block
                    if let Some(mut fj) = fj {
                        //if the next block we would enter needs to be jumped over, we set the score to how close we are to it.
                        let delta = *dest - tf.translation();
                        let origin = BlockCoord::from(tf.translation()) + BlockCoord::new(0, 1, 0);
                        let tf_origin = tf.translation();
                        let mut closest_distance: f32 = 1.0;
                        //test the 8 neighbors one block above us
                        // x x x
                        // x   x
                        // x x x
                        if delta.x > 0.0 {
                            match level
                                .get_block_entity(origin + BlockCoord::new(1, 0, 0))
                                .map(|b| block_physics.get(b).ok())
                                .flatten()
                            {
                                Some(BlockPhysics::Empty) | None => {}
                                _ => {
                                    closest_distance =
                                        closest_distance.min(tf_origin.x.ceil() - tf_origin.x);
                                }
                            }
                            //do +x corners
                            if delta.z > 0.0 {
                                match level
                                    .get_block_entity(origin + BlockCoord::new(1, 0, 1))
                                    .map(|b| block_physics.get(b).ok())
                                    .flatten()
                                {
                                    Some(BlockPhysics::Empty) | None => {}
                                    _ => {
                                        closest_distance = closest_distance.min(
                                            Vec2::new(tf_origin.x.ceil(), tf_origin.y.ceil())
                                                .distance(Vec2::new(tf_origin.x, tf_origin.z)),
                                        );
                                    }
                                }
                            } else if delta.z < 0.0 {
                                match level
                                    .get_block_entity(origin + BlockCoord::new(1, 0, -1))
                                    .map(|b| block_physics.get(b).ok())
                                    .flatten()
                                {
                                    Some(BlockPhysics::Empty) | None => {}
                                    _ => {
                                        closest_distance = closest_distance.min(
                                            Vec2::new(tf_origin.x.ceil(), tf_origin.y.floor())
                                                .distance(Vec2::new(tf_origin.x, tf_origin.z)),
                                        );
                                    }
                                }
                            }
                        } else if delta.x < 0.0 {
                            match level
                                .get_block_entity(origin + BlockCoord::new(-1, 0, 0))
                                .map(|b| block_physics.get(b).ok())
                                .flatten()
                            {
                                Some(BlockPhysics::Empty) | None => {}
                                _ => {
                                    closest_distance =
                                        closest_distance.min(tf_origin.x - tf_origin.x.floor());
                                }
                            }
                            //do -x corners
                            if delta.z > 0.0 {
                                match level
                                    .get_block_entity(origin + BlockCoord::new(-1, 0, 1))
                                    .map(|b| block_physics.get(b).ok())
                                    .flatten()
                                {
                                    Some(BlockPhysics::Empty) | None => {}
                                    _ => {
                                        closest_distance = closest_distance.min(
                                            Vec2::new(tf_origin.x.floor(), tf_origin.y.ceil())
                                                .distance(Vec2::new(tf_origin.x, tf_origin.z)),
                                        );
                                    }
                                }
                            } else if delta.z < 0.0 {
                                match level
                                    .get_block_entity(origin + BlockCoord::new(-1, 0, -1))
                                    .map(|b| block_physics.get(b).ok())
                                    .flatten()
                                {
                                    Some(BlockPhysics::Empty) | None => {}
                                    _ => {
                                        closest_distance = closest_distance.min(
                                            Vec2::new(tf_origin.x.floor(), tf_origin.y.floor())
                                                .distance(Vec2::new(tf_origin.x, tf_origin.z)),
                                        );
                                    }
                                }
                            }
                        }
                        if delta.z > 0.0 {
                            match level
                                .get_block_entity(origin + BlockCoord::new(0, 0, 1))
                                .map(|b| block_physics.get(b).ok())
                                .flatten()
                            {
                                Some(BlockPhysics::Empty) | None => {}
                                _ => {
                                    closest_distance =
                                        closest_distance.min(tf_origin.z.ceil() - tf_origin.z);
                                }
                            }
                        } else if delta.z < 0.0 {
                            match level
                                .get_block_entity(origin + BlockCoord::new(0, 0, -1))
                                .map(|b| block_physics.get(b).ok())
                                .flatten()
                            {
                                Some(BlockPhysics::Empty) | None => {}
                                _ => {
                                    closest_distance =
                                        closest_distance.min(tf_origin.z - tf_origin.z.floor());
                                }
                            }
                        }
                        if closest_distance < JUMP_DIST {
                            fj.0 = true;
                            //set a wait time so we don't immedately grab on to the block we are trying to jump over
                            action.wait_timer = Timer::new(JUMP_COOLDOWN, TimerMode::Once);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
