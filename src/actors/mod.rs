use bevy::prelude::*;
use big_brain::prelude::*;

mod player;
pub use player::*;

mod combat;
pub use combat::*;

use self::personality::PersonalityPlugin;

pub mod glowjelly;

pub mod personality;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CombatPlugin)
            .add_plugin(BigBrainPlugin)
            .add_plugin(PersonalityPlugin)
            .add_plugin(glowjelly::GlowjellyPlugin)
            .add_plugin(player::PlayerPlugin)
            .add_system(idle_action_system)
        ;
    }
}

#[derive(Component)]
pub struct MoveSpeed {
    pub base_accel: f32,
    pub current_accel: f32,
    pub max_speed: f32,
}

impl Default for MoveSpeed {
    fn default() -> Self {
        MoveSpeed {
                base_accel: 75.0,
                current_accel: 75.0,
                max_speed: 100.0,
            }
    }
}

#[derive(Component)]
pub struct Jump {
    pub base_height: f32,
    pub current_height: f32,
    //you get 1 jump if you're on the ground + extra_jump_count jumps you can use in the air
    pub extra_jumps_remaining: u32,
    pub extra_jump_count: u32
}

impl Default for Jump {
    fn default() -> Self {
        Jump { base_height: 6.0, current_height: 6.0, extra_jumps_remaining: 100, extra_jump_count: 100}
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct UninitializedActor;

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct IdleAction {
    pub seconds: f32,
}

#[derive(Component, Debug, Default)]
pub struct Idler {
    pub seconds_remaining: f32
}

fn idle_action_system (
    time: Res<Time>,
    mut info: Query<&mut Idler>,
    mut actor: Query<(&Actor, &mut ActionState, &IdleAction)>
) {
    for (Actor(actor), mut state, action) in actor.iter_mut() {
        if let Ok(mut idle) = info.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    *state = ActionState::Executing;
                    idle.seconds_remaining = action.seconds;
                },
                ActionState::Executing => {
                    idle.seconds_remaining -= time.delta_seconds();
                    if idle.seconds_remaining <= 0.0 {
                        *state = ActionState::Success;
                    }
                },
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}