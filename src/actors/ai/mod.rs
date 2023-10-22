use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier3d::na::ComplexField;
use big_brain::prelude::*;
use rand_distr::num_traits::Float;

use crate::{
    controllers::{FrameJump, FrameMovement},
    util::plugin::SmoothLookTo,
    world::{BlockCoord, BlockPhysics, Level, LevelSystemSet},
};

pub mod scorers;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(scorers::ScorersPlugin).add_systems(
            Update,
            walk_to_destination_action
                .in_set(BigBrainSet::Actions)
                .in_set(LevelSystemSet::Main),
        );
    }
}

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct WalkToDestinationAction {
    pub target_dest: Vec3,
    pub stop_distance: f32,
}

#[derive(Component, Debug, ActionBuilder, Copy, Clone)]
pub struct AttackAction;

impl Default for WalkToDestinationAction {
    fn default() -> Self {
        Self {
            target_dest: Vec3::default(),
            stop_distance: 0.1,
        }
    }
}

fn walk_to_destination_action(
    mut info: Query<(
        &Transform,
        &mut FrameMovement,
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
                ActionState::Executing | ActionState::Cancelled => {
                    let dest = action.target_dest;
                    let delta = Vec3::new(dest.x, 0.0, dest.z)
                        - Vec3::new(tf.translation.x, 0.0, tf.translation.z);

                    if delta.length_squared() < action.stop_distance * action.stop_distance {
                        //we are close enough
                        *state = ActionState::Success;
                        return;
                    }
                    
                    fm.0 = delta;
                    let delta_normed = delta.normalize_or_zero();
                    if let Some(mut look) = look_opt {
                        look.enabled = true;
                        look.up = Vec3::Y;
                        look.forward = delta_normed;
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
            .map(|b| block_physics.get(b).ok())
            .flatten()
        {
            Some(BlockPhysics::Empty) | None => {}
            _ => {
                return Some((distance, coord));
            }
        }
    }
    return None;
}
