use bevy::prelude::*;

#[derive(Component)]
pub struct Player{
    pub selected_slot: u32,
    pub hit_damage: f32,
}

#[derive(Component)]
pub struct LocalPlayer{}