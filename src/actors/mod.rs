use bevy::prelude::*;
use big_brain::prelude::*;

mod player;
pub use player::*;

mod combat;
pub use combat::*;

pub mod glowjelly;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CombatPlugin)
            .add_plugin(BigBrainPlugin)
            .add_plugin(glowjelly::GlowjellyPlugin);
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
        Jump { base_height: 6.0, current_height: 6.0, extra_jumps_remaining: 10, extra_jump_count: 10}
    }
}