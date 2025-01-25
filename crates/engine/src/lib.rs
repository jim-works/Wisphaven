//disable console window from popping up on windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//have to enable this because it's a nursery feature
#![warn(clippy::disallowed_types)]
//bevy system signatures often violate these rules
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//TODO: remove this before release. annoying as balls during development
#![allow(dead_code)]
#![feature(assert_matches)]
#![feature(let_chains)]
#![feature(trivial_bounds)]

pub mod actors;
pub mod camera;
pub mod chunk_loading;
pub mod controllers;
pub mod debug;
pub mod effects;
pub mod items;
pub mod mesher;
pub mod net;
pub mod physics;
pub mod serialization;
pub mod state;
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
    GameOver,
}

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            util::plugin::UtilPlugin,
            serialization::SerializationPlugin,
            world::LevelPlugin,
            mesher::MesherPlugin,
            worldgen::WorldGenPlugin,
            chunk_loading::ChunkLoaderPlugin,
            physics::PhysicsPlugin,
            controllers::ControllersPlugin,
            actors::ActorPlugin,
            items::ItemsPlugin,
            net::NetPlugin,
            debug::DebugUIPlugin,
            effects::EffectsPlugin,
            camera::CameraPlugin,
            state::GameStatePlugin,
        ));
    }
}
