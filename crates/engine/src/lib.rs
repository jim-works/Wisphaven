pub mod actors;
pub mod chunk_loading;
pub mod controllers;
pub mod effects;
pub mod gameplay;
pub mod items;
pub mod mesher;
pub mod net;
pub mod physics;
pub mod serialization;
pub mod ui;
pub mod world;
pub mod worldgen;

use ::util;
use bevy::prelude::*;

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameState {
    #[default]
    Setup,
    Menu,
    Game,
}
