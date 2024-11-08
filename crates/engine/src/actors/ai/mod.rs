use std::time::Duration;

use bevy::prelude::*;
use big_brain::prelude::*;

use crate::{
    controllers::{FrameJump, TickMovement},
    util::plugin::SmoothLookTo,
    world::{BlockCoord, BlockPhysics, Level, LevelSystemSet},
};

use super::AggroTargets;

pub mod scorers;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(scorers::ScorersPlugin).add_systems(
            Update,
            (
                walk_to_destination_action,
                walk_to_entity_action,
                walk_to_current_target_action,
                fly_to_current_target_action,
            )
                .in_set(BigBrainSet::Actions)
                .in_set(LevelSystemSet::Main),
        );
    }
}

#[derive(Component, Debug, ActionBuilder, Copy, Clone)]
pub struct AttackAction;

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct WalkToDestinationAction {
    pub target_dest: Vec3,
    pub stop_distance: f32,
    pub look_in_direction: bool,
}

impl Default for WalkToDestinationAction {
    fn default() -> Self {
        Self {
            target_dest: Vec3::default(),
            stop_distance: 1.0,
            look_in_direction: true,
        }
    }
}

fn walk_to_destination_action(
    mut info: Query<(
        &Transform,
        &mut TickMovement,
        Option<&mut FrameJump>,
        Option<&mut SmoothLookTo>,
    )>,
    mut query: Query<(&Actor, &mut ActionState, &mut WalkToDestinationAction)>,
    level: Res<Level>,
    block_physics: Query<&BlockPhysics>,
) {
    const JUMP_DIST: f32 = 0.75;
    const JUMP_COOLDOWN: Duration = Duration::from_millis(500);
    for (Actor(actor), mut state, action) in query.iter_mut() {
        if let Ok((tf, mut fm, fj, look_opt)) = info.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    let dest = action.target_dest;
                    let delta = Vec3::new(dest.x, 0.0, dest.z)
                        - Vec3::new(tf.translation.x, 0.0, tf.translation.z);

                    if delta.length_squared() < action.stop_distance * action.stop_distance {
                        //we are close enough
                        *state = ActionState::Success;
                        if action.look_in_direction {
                            if let Some(mut look) = look_opt {
                                look.enabled = false;
                            }
                        }
                        fm.0 = Vec3::ZERO;
                        return;
                    }

                    fm.0 = delta;
                    let delta_normed = delta.normalize_or_zero();
                    if action.look_in_direction {
                        if let Some(mut look) = look_opt {
                            look.up = Vec3::Y;
                            look.forward = delta_normed;
                            look.enabled = true;
                        }
                    }

                    //test if we need to jump over a block
                    if let Some(mut fj) = fj {
                        if get_closest_block_dist(
                            Vec2::new(delta_normed.x, delta_normed.z),
                            tf.translation,
                            &level,
                            &block_physics,
                        )
                        .map(|(d, _)| d < JUMP_DIST)
                        .unwrap_or(false)
                        {
                            fj.0 = true;
                        }
                    }
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                    if action.look_in_direction {
                        if let Some(mut look) = look_opt {
                            look.enabled = false;
                        }
                    }
                    fm.0 = Vec3::ZERO;
                }
                _ => {}
            }
        } else {
            *state = ActionState::Failure;
        }
    }
}

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct WalkToEntityAction {
    pub target_entity: Entity,
    pub stop_distance: f32,
    pub look_in_direction: bool,
}

impl Default for WalkToEntityAction {
    fn default() -> Self {
        Self {
            target_entity: Entity::PLACEHOLDER,
            stop_distance: 1.0,
            look_in_direction: true,
        }
    }
}

fn walk_to_entity_action(
    mut info: Query<(
        &Transform,
        &mut TickMovement,
        Option<&mut FrameJump>,
        Option<&mut SmoothLookTo>,
    )>,
    mut query: Query<(&Actor, &mut ActionState, &mut WalkToEntityAction)>,
    level: Res<Level>,
    block_physics: Query<&BlockPhysics>,
    tf_query: Query<&GlobalTransform>,
) {
    const JUMP_DIST: f32 = 0.75;
    for (Actor(actor), mut state, action) in query.iter_mut() {
        if let Ok(target_tf) = tf_query.get(action.target_entity) {
            if let Ok((tf, mut fm, fj, look_opt)) = info.get_mut(*actor) {
                match *state {
                    ActionState::Requested => {
                        *state = ActionState::Executing;
                    }
                    ActionState::Executing => {
                        let dest = target_tf.translation();
                        let delta = Vec3::new(dest.x, 0.0, dest.z)
                            - Vec3::new(tf.translation.x, 0.0, tf.translation.z);

                        if delta.length_squared() < action.stop_distance * action.stop_distance {
                            //we are close enough
                            *state = ActionState::Success;
                            if action.look_in_direction {
                                if let Some(mut look) = look_opt {
                                    look.enabled = false;
                                }
                            }
                            fm.0 = Vec3::ZERO;
                            return;
                        }

                        fm.0 = delta;
                        let delta_normed = delta.normalize_or_zero();
                        if action.look_in_direction {
                            if let Some(mut look) = look_opt {
                                look.up = Vec3::Y;
                                look.forward = delta_normed;
                                look.enabled = true;
                            }
                        }

                        //test if we need to jump over a block
                        if let Some(mut fj) = fj {
                            if get_closest_block_dist(
                                Vec2::new(delta_normed.x, delta_normed.z),
                                tf.translation,
                                &level,
                                &block_physics,
                            )
                            .map(|(d, _)| d < JUMP_DIST)
                            .unwrap_or(false)
                            {
                                fj.0 = true;
                            }
                        }
                    }
                    ActionState::Cancelled => {
                        *state = ActionState::Failure;
                        if action.look_in_direction {
                            if let Some(mut look) = look_opt {
                                look.enabled = false;
                            }
                        }
                        fm.0 = Vec3::ZERO;
                    }
                    _ => {}
                }
            } else {
                *state = ActionState::Failure;
            }
        } else {
            //no tf on the target entity to move to
            *state = ActionState::Failure;
        }
    }
}

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct WalkToCurrentTargetAction {
    pub stop_distance: f32,
    pub look_in_direction: bool,
}

impl Default for WalkToCurrentTargetAction {
    fn default() -> Self {
        Self {
            stop_distance: 1.0,
            look_in_direction: true,
        }
    }
}

fn walk_to_current_target_action(
    mut info: Query<(
        &Transform,
        &AggroTargets,
        &mut TickMovement,
        Option<&mut FrameJump>,
        Option<&mut SmoothLookTo>,
    )>,
    mut query: Query<(&Actor, &mut ActionState, &mut WalkToCurrentTargetAction)>,
    level: Res<Level>,
    block_physics: Query<&BlockPhysics>,
    tf_query: Query<&GlobalTransform>,
) {
    const JUMP_DIST: f32 = 0.75;
    for (Actor(actor), mut state, action) in query.iter_mut() {
        if let Ok((tf, targets, mut fm, fj, look_opt)) = info.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    if let Some(target_tf) =
                        targets.current_target().and_then(|e| tf_query.get(e).ok())
                    {
                        let dest = target_tf.translation();
                        let delta = Vec3::new(dest.x, 0.0, dest.z)
                            - Vec3::new(tf.translation.x, 0.0, tf.translation.z);

                        if delta.length_squared() < action.stop_distance * action.stop_distance {
                            //we are close enough
                            *state = ActionState::Success;
                            if action.look_in_direction {
                                if let Some(mut look) = look_opt {
                                    look.enabled = false;
                                }
                            }
                            fm.0 = Vec3::ZERO;
                            return;
                        }

                        fm.0 = delta;
                        let delta_normed = delta.normalize_or_zero();
                        if action.look_in_direction {
                            if let Some(mut look) = look_opt {
                                look.up = Vec3::Y;
                                look.forward = delta_normed;
                                look.enabled = true;
                            }
                        }

                        //test if we need to jump over a block
                        if let Some(mut fj) = fj {
                            if get_closest_block_dist(
                                Vec2::new(delta_normed.x, delta_normed.z),
                                tf.translation,
                                &level,
                                &block_physics,
                            )
                            .map(|(d, _)| d < JUMP_DIST)
                            .unwrap_or(false)
                            {
                                fj.0 = true;
                            }
                        }
                    } else {
                        //no tf on the target entity to move to
                        if action.look_in_direction {
                            if let Some(mut look) = look_opt {
                                look.enabled = false;
                            }
                        }
                        *state = ActionState::Failure;
                    }
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                    if action.look_in_direction {
                        if let Some(mut look) = look_opt {
                            look.enabled = false;
                        }
                    }
                    fm.0 = Vec3::ZERO;
                }
                _ => {}
            }
        }
    }
}

//returns distance to the closest solid block in the surrounding blocks in direction dir, and its position.
//dir SHOULD BE A UNIT VECTOR!
//consider the 8 neighbors surrounding jump_test_origin
// x x x
// x   x
// x x x
//test in the direction of dir. if delta is diagonal (ex 1,1), test all 3 blocks. otherwise, just test delta
fn get_closest_block_dist(
    dir: Vec2,
    jump_test_origin: Vec3,
    level: &Level,
    block_physics: &Query<&BlockPhysics>,
) -> Option<(f32, BlockCoord)> {
    if dir.x.abs() < f32::EPSILON || dir.y.abs() < f32::EPSILON {
        return None;
    }
    //test if we need to jump over a block
    let delta = BlockCoord::new(
        if dir.x > 0. {
            1
        } else if dir.x < 0. {
            -1
        } else {
            0
        },
        0,
        if dir.y > 0. {
            1
        } else if dir.y < 0. {
            -1
        } else {
            0
        },
    );
    let origin = BlockCoord::from(jump_test_origin) + BlockCoord::new(0, 1, 0);
    let pos_in_square = Vec2::new(
        jump_test_origin.x - jump_test_origin.x.floor(),
        jump_test_origin.z - jump_test_origin.z.floor(),
    );
    //test blocks in order of closeness
    //diagonal will always be furthest away
    let mut test_blocks = [BlockCoord::new(0, 0, 0); 3];
    let mut distances = [0.; 3];
    if pos_in_square.x.abs() < pos_in_square.y.abs() {
        test_blocks[0] = origin + BlockCoord::new(delta.x, 0, 0);
        //distance to x wall of square in direction of dir
        distances[0] = (dir.x.ceil() - pos_in_square.x).abs();
        test_blocks[1] = origin + BlockCoord::new(0, 0, delta.z);
        //distance to z wall of square in direction of dir
        distances[1] = (dir.y.ceil() - pos_in_square.y).abs();
        test_blocks[2] = origin + delta;
        //distance to corner of square in direction of dir
        distances[2] = Vec2::new(dir.x.ceil(), dir.y.ceil()).distance(pos_in_square);
    } else {
        test_blocks[0] = origin + BlockCoord::new(0, 0, delta.z);
        //distance to z wall of square in direction of dir
        distances[0] = (dir.y.ceil() - pos_in_square.y).abs();
        test_blocks[1] = origin + BlockCoord::new(delta.x, 0, 0);
        //distance to x wall of square in direction of dir
        distances[1] = (dir.x.ceil() - pos_in_square.x).abs();
        test_blocks[2] = origin + delta;
        //distance to corner of square in direction of dir
        distances[2] = Vec2::new(dir.x.ceil(), dir.y.ceil()).distance(pos_in_square);
    }
    for (distance, coord) in distances.into_iter().zip(test_blocks.into_iter()) {
        match level
            .get_block_entity(coord)
            .and_then(|b| block_physics.get(b).ok())
        {
            Some(BlockPhysics::Empty) | None => {}
            _ => {
                return Some((distance, coord));
            }
        }
    }
    None
}

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct FlyToCurrentTargetAction {
    pub stop_distance: f32,
    pub look_in_direction: bool,
    pub offset: Vec3,
}

impl Default for FlyToCurrentTargetAction {
    fn default() -> Self {
        Self {
            stop_distance: 1.,
            look_in_direction: true,
            offset: Vec3::Y * 5.,
        }
    }
}

fn fly_to_current_target_action(
    mut info: Query<(
        &Transform,
        &AggroTargets,
        &mut TickMovement,
        Option<&mut SmoothLookTo>,
    )>,
    mut query: Query<(&Actor, &mut ActionState, &mut FlyToCurrentTargetAction)>,
    tf_query: Query<&GlobalTransform>,
    name_query: Query<&Name>,
) {
    fn cleanup(
        action: &FlyToCurrentTargetAction,
        look_opt: &mut Option<Mut<SmoothLookTo>>,
        fm: &mut TickMovement,
    ) {
        if action.look_in_direction {
            if let Some(ref mut look) = look_opt {
                look.enabled = false;
            }
        }
        fm.0 = Vec3::ZERO;
    }
    for (Actor(actor), mut state, action) in query.iter_mut() {
        let Ok((tf, targets, mut fm, mut look_opt)) = info.get_mut(*actor) else {
            warn!("Entity with FlyToCurrentTargetAction doesn't satisfy the necessary query.");
            continue;
        };
        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Some(target_tf) = targets.current_target().and_then(|e| tf_query.get(e).ok())
                else {
                    *state = ActionState::Cancelled;
                    if targets.current_target().is_some() {
                        warn!("target entity doesn't have GlobalTransform");
                    }
                    continue;
                };
                let delta = target_tf.translation() - tf.translation;
                if delta.length_squared() < action.stop_distance * action.stop_distance {
                    *state = ActionState::Success;
                    cleanup(&action, &mut look_opt, &mut fm);
                }
                let delta_normed = delta.normalize_or_zero();
                fm.0 = delta_normed;
                if action.look_in_direction {
                    if let Some(mut look) = look_opt {
                        look.up = Vec3::Y;
                        look.forward = delta_normed;
                        look.enabled = true;
                    }
                }
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
                cleanup(&action, &mut look_opt, &mut fm);
            }
            _ => {}
        }
    }
}
