use bevy::prelude::*;

#[derive(Component)]
pub struct Player{
    pub hit_damage: f32,
}

#[derive(Component)]
pub struct LocalPlayer;