use bevy::prelude::*;

mod player;
pub use player::*;

#[derive(Component)]
pub struct MoveSpeed {
    pub base: f32,
    pub current: f32,
    pub max: f32,
}

#[derive(Component)]
pub struct Jump {
    pub base: f32,
    pub current: f32,
    //you get 1 jump if you're on the ground + extra_jump_count jumps you can use in the air
    pub extra_jumps_remaining: u32,
    pub extra_jump_count: u32
}

impl Default for Jump {
    fn default() -> Self {
        Jump { base: 10.0, current: 10.0, extra_jumps_remaining: 1, extra_jump_count: 1}
    }
}