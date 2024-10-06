use bevy::{prelude::*, utils::HashMap};
use big_brain::prelude::*;

mod player;
pub use player::*;

mod combat;
pub use combat::*;
use serde::{Deserialize, Serialize};

use crate::util::{ease_out_quad, inverse_lerp, lerp};

pub mod abilities;
pub mod ai;
pub mod block_actors;
pub mod ghost;
pub mod world_anchor;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CombatPlugin,
            BigBrainPlugin::new(PreUpdate),
            block_actors::BlockActorPlugin,
            world_anchor::WorldAnchorPlugin,
            player::PlayerPlugin,
            ai::AIPlugin,
            ghost::GhostPlugin,
            abilities::AbilityPlugin,
        ))
        .add_systems(Update, idle_action_system)
        .insert_resource(ActorResources {
            registry: ActorRegistry::default(),
        })
        .register_type::<ActorName>();
    }
}

#[derive(Component)]
pub struct MoveSpeed {
    pub no_input_accel: f32,
    pub grounded_accel: f32,
    pub aerial_accel: f32,
    //extra acceleration when moving in directly opposite to velocity (makes turns snappier)
    pub turn_accel_mult: f32,
    //multiplier, applied after accel_add
    pub accel_mult: f32,
    //added before accel_mult is applied
    pub accel_add: f32,
    pub max_speed: f32,
}

impl Default for MoveSpeed {
    fn default() -> Self {
        MoveSpeed {
            no_input_accel: 0.1,
            grounded_accel: 0.5,
            aerial_accel: 0.2,
            accel_mult: 1.0,
            accel_add: 0.0,
            max_speed: 0.1,
            turn_accel_mult: 2.0,
        }
    }
}

impl MoveSpeed {
    pub fn new(grounded_accel: f32, aerial_accel: f32, max_speed: f32) -> Self {
        Self {
            grounded_accel,
            aerial_accel,
            max_speed,
            ..default()
        }
    }

    pub fn get_accel(
        &self,
        grounded: bool,
        has_input: bool,
        norm_target_v: Vec3,
        norm_current_v: Vec3,
    ) -> f32 {
        const EPSILON: f32 = 0.001;
        let base_speed = if !has_input {
            self.no_input_accel
        } else if grounded {
            self.grounded_accel
        } else {
            self.aerial_accel
        };
        let mut opposite_factor =
            inverse_lerp(1.0, -1.0, norm_target_v.dot(norm_current_v)).clamp(0.0, 1.0);
        if !opposite_factor.is_finite() {
            opposite_factor = 0.5; //assume v is 0, which would be 0 dot product, which is halfway between -1 and 1
        }
        let turn_accel_mult = lerp(1.0, self.turn_accel_mult, ease_out_quad(opposite_factor));
        (base_speed + self.accel_add) * self.accel_mult * turn_accel_mult
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
            base_height: 0.125,
            current_height: 0.125,
            extra_jumps_remaining: 1,
            extra_jump_count: 1,
        }
    }
}

impl Jump {
    pub fn new(height: f32, extra_jumps: u32) -> Self {
        Self {
            base_height: height,
            current_height: height,
            extra_jumps_remaining: extra_jumps,
            extra_jump_count: extra_jumps,
        }
    }
}

#[derive(Component)]
pub struct DefaultAnimation {
    anim: AnimationNodeIndex,
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
        anim: AnimationNodeIndex,
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
    setup_animation_with_speed(anim_opt, animation_player, None);
}

pub fn setup_animation_with_speed(
    anim_opt: Option<Mut<'_, DefaultAnimation>>,
    animation_player: &mut Query<&mut AnimationPlayer>,
    speed: Option<f32>,
) {
    if let Some(mut anim) = anim_opt {
        anim.reset();
        if let Some(speed) = speed {
            anim.animation_speed = speed;
        }
        if let Ok(mut anim_player) = animation_player.get_mut(anim.player) {
            let active_anim = anim_player.start(anim.anim);
            if let Some(speed) = speed {
                active_anim.set_speed(speed);
            }
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
#[reflect(Component, FromWorld)]
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
    //should be fine not being behind an arc, can probably send an event if it needs to be done async during loading
    //easier to find a work around for that than a workaround for not being able to mutate it to add new actors during loading
    //  ...that is until we get prefabs
    pub registry: ActorRegistry,
}

pub type ActorNameIdMap = HashMap<ActorName, ActorId>;

#[derive(Default)]
pub struct ActorRegistry {
    pub dynamic_generators: Vec<Box<dyn Fn(&mut Commands, Transform) + Send + Sync>>,
    //ids may not be stable across program runs
    pub id_map: ActorNameIdMap,
}

impl ActorRegistry {
    pub fn add_dynamic(
        &mut self,
        name: ActorName,
        generator: Box<dyn Fn(&mut Commands, Transform) + Send + Sync>,
    ) {
        let id = ActorId(self.dynamic_generators.len());
        self.dynamic_generators.push(generator);
        self.id_map.insert(name, id);
    }
    pub fn get_id(&self, name: &ActorName) -> Option<ActorId> {
        self.id_map.get(name).copied()
    }
    pub fn spawn(&self, actor: &ActorName, commands: &mut Commands, spawn_tf: Transform) {
        if let Some(actor_id) = self.get_id(actor) {
            if let Some(gen) = self.dynamic_generators.get(actor_id.0) {
                gen(commands, spawn_tf);
            }
        }
    }
    pub fn spawn_id(&self, actor_id: ActorId, commands: &mut Commands, spawn_tf: Transform) {
        if let Some(gen) = self.dynamic_generators.get(actor_id.0) {
            gen(commands, spawn_tf);
        }
    }
}
