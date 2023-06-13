use bevy::prelude::*;

#[derive(Component)]
pub struct Player{
    pub selected_slot: u32
}

#[derive(Component)]
pub struct LocalPlayer{}