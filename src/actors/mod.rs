use bevy::prelude::*;
use big_brain::prelude::*;

mod player;
pub use player::*;

mod combat;
pub use combat::*;

use self::personality::PersonalityPlugin;

pub mod glowjelly;
pub mod personality;
pub mod behaviors;

#[cfg(test)]
mod test;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CombatPlugin)
            .add_plugin(BigBrainPlugin)
            .add_plugin(PersonalityPlugin)
            .add_plugin(behaviors::BehaviorsPlugin)
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
pub struct DefaultAnimation {
    anim: Handle<AnimationClip>,
    player: Entity,
    action_time: f32,
    duration: f32,
    animation_speed: f32,
    acted: bool,
    just_acted: bool,
    time_elapsed: f32,
}

impl DefaultAnimation {
    pub fn reset(&mut self) {
        self.acted = false;
        self.time_elapsed = 0.0;
    }
    pub fn tick(&mut self, dt: f32) {
        self.time_elapsed += dt;
        self.just_acted = !self.acted && self.time_elapsed >= self.action_seconds();
        self.acted = self.time_elapsed >= self.action_seconds();
    }
    pub fn scaled_time(&self, time: f32) -> f32 {
        if self.animation_speed == 0.0 {0.0} else {time/self.animation_speed}
    }
    pub fn duration_seconds(&self) -> f32 {
        self.scaled_time(self.duration)
    }
    pub fn action_seconds(&self) -> f32 {
        self.scaled_time(self.action_time)
    }
    pub fn finished(&self) -> bool {
        self.time_elapsed >= self.duration_seconds()
    }
    pub fn just_acted(&self) -> bool {
        self.just_acted
    }
    pub fn new(anim: Handle<AnimationClip>, player: Entity, action_time: f32, duration_seconds: f32) -> Self {
        Self {
            anim,
            player,
            action_time,
            duration: duration_seconds,
            animation_speed: 1.0,
            acted: false,
            time_elapsed: 0.0,
            just_acted: false,
        }
    }
}

pub fn setup_animation(anim_opt: Option<Mut<'_, DefaultAnimation>>, animation_player: &mut Query<&mut AnimationPlayer>) {
    setup_animation_with_speed(anim_opt, animation_player, 1.0);
}

pub fn setup_animation_with_speed(anim_opt: Option<Mut<'_, DefaultAnimation>>, animation_player: &mut Query<&mut AnimationPlayer>, speed: f32) {
    if let Some(mut anim) = anim_opt {
        if let Ok(mut anim_player) = animation_player.get_mut(anim.player) {
            anim_player.start(anim.anim.clone_weak());
            anim_player.set_speed(speed);
            anim.animation_speed = speed;
            anim.reset();
        }
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