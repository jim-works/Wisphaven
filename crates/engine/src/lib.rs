//disable console window from popping up on windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//have to enable this because it's a nursery feature
#![warn(clippy::disallowed_types)]
//bevy system signatures often violate these rules
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//TODO: remove this before release. annoying as balls during development
#![allow(dead_code)]
//don't care too much about precision here, so I'll allow this
#![feature(const_fn_floating_point_arithmetic)]
#![feature(assert_matches)]

pub mod actors;
pub mod chunk_loading;
pub mod controllers;
pub mod debug;
pub mod effects;
pub mod gameplay;
pub mod items;
pub mod mesher;
pub mod net;
pub mod physics;
pub mod serialization;
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
            gameplay::GameplayPlugin,
            debug::DebugUIPlugin,
            effects::EffectsPlugin,
        ))
        .init_state::<GameState>();
    }
}
