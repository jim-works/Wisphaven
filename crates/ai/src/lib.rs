#![feature(let_chains)]

pub mod attacker;

use bevy::prelude::*;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(attacker::AttackerAIPlugin);
    }
}
