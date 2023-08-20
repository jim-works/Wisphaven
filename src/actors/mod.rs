use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};
use big_brain::prelude::*;

mod player;
pub use player::*;

mod combat;
pub use combat::*;
use serde::{Deserialize, Serialize};

use crate::util::SendEventCommand;

use self::{glowjelly::SpawnGlowjellyEvent, personality::PersonalityPlugin};

pub mod behaviors;
pub mod block_actors;
pub mod glowjelly;
pub mod personality;

#[cfg(test)]
mod test;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = ActorRegistry::default();
        registry.add_dynamic(
            ActorName::core("glowjelly"),
            Box::new(|commands, tf| {
                commands.add(SendEventCommand(SpawnGlowjellyEvent {
                    location: tf,
                    color: Color::RED,
                }))
            }),
        );
        app.add_plugins(CombatPlugin)
            .add_plugins(BigBrainPlugin::new(PreUpdate))
            .add_plugins(PersonalityPlugin)
            .add_plugins(block_actors::BlockActorPlugin)
            .add_plugins(behaviors::BehaviorsPlugin)
            .add_plugins(glowjelly::GlowjellyPlugin)
            .add_plugins(player::PlayerPlugin)
            .add_systems(Update, idle_action_system)
            .insert_resource(ActorResources {
                registry: Arc::new(registry),
            })
            .register_type::<ActorName>();
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
    pub extra_jump_count: u32,
}

impl Default for Jump {
    fn default() -> Self {
        Jump {
            base_height: 6.0,
            current_height: 6.0,
            extra_jumps_remaining: 100,
            extra_jump_count: 100,
        }
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
        if self.animation_speed == 0.0 {
            0.0
        } else {
            time / self.animation_speed
        }
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
    pub fn new(
        anim: Handle<AnimationClip>,
        player: Entity,
        action_time: f32,
        duration_seconds: f32,
    ) -> Self {
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

pub fn setup_animation(
    anim_opt: Option<Mut<'_, DefaultAnimation>>,
    animation_player: &mut Query<&mut AnimationPlayer>,
) {
    setup_animation_with_speed(anim_opt, animation_player, 1.0);
}

pub fn setup_animation_with_speed(
    anim_opt: Option<Mut<'_, DefaultAnimation>>,
    animation_player: &mut Query<&mut AnimationPlayer>,
    speed: f32,
) {
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
    pub seconds_remaining: f32,
}

fn idle_action_system(
    time: Res<Time>,
    mut info: Query<&mut Idler>,
    mut actor: Query<(&Actor, &mut ActionState, &IdleAction)>,
) {
    for (Actor(actor), mut state, action) in actor.iter_mut() {
        if let Ok(mut idle) = info.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    *state = ActionState::Executing;
                    idle.seconds_remaining = action.seconds;
                }
                ActionState::Executing => {
                    idle.seconds_remaining -= time.delta_seconds();
                    if idle.seconds_remaining <= 0.0 {
                        *state = ActionState::Success;
                    }
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

#[derive(
    Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component)]
pub struct ActorName {
    pub namespace: String,
    pub name: String,
}

impl ActorName {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }
    pub fn core(name: impl Into<String>) -> Self {
        Self::new("core", name)
    }
}

//actor ids may not be stable across program runs. to get a specific id for an actor,
// use actor registry
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub usize);

#[derive(Bundle)]
pub struct ActorBundle {
    pub name: ActorName,
}

#[derive(Resource)]
pub struct ActorResources {
    pub registry: Arc<ActorRegistry>,
}

pub type ActorNameIdMap = HashMap<ActorName, ActorId>;

#[derive(Default)]
pub struct ActorRegistry {
    pub dynamic_generators: Vec<Box<dyn Fn(&mut Commands, GlobalTransform) + Send + Sync>>,
    //ids may not be stable across program runs
    pub id_map: ActorNameIdMap,
}

impl ActorRegistry {
    pub fn add_dynamic(
        &mut self,
        name: ActorName,
        generator: Box<dyn Fn(&mut Commands, GlobalTransform) + Send + Sync>,
    ) {
        let id = ActorId(self.dynamic_generators.len());
        self.dynamic_generators.push(generator);
        self.id_map.insert(name, id);
    }
    pub fn get_id(&self, name: &ActorName) -> Option<ActorId> {
        self.id_map.get(name).copied()
    }
    pub fn spawn(&self, actor: &ActorName, commands: &mut Commands, spawn_tf: GlobalTransform) {
        if let Some(actor_id) = self.get_id(actor) {
            if let Some(gen) = self.dynamic_generators.get(actor_id.0) {
                gen(commands, spawn_tf);
            }
        }
    }
    pub fn spawn_id(&self, actor_id: ActorId, commands: &mut Commands, spawn_tf: GlobalTransform) {
        if let Some(gen) = self.dynamic_generators.get(actor_id.0) {
            gen(commands, spawn_tf);
        }
    }
}
