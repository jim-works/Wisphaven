//disable console window from popping up on windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//have to enable this because it's a nursery feature
#![warn(clippy::disallowed_types)]
//bevy system signatures often violate these rules
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//TODO: remove this before release. annoying as balls during development
#![allow(dead_code)]



use actors::ActorPlugin;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use chunk_loading::{ChunkLoader, ChunkLoaderPlugin};
use controllers::ControllersPlugin;
use items::ItemsPlugin;

use mesher::MesherPlugin;
use physics::PhysicsPlugin;
use util::plugin::UtilPlugin;
use world::*;
use worldgen::WorldGenPlugin;

mod actors;
mod chunk_loading;
mod controllers;
mod mesher;
mod physics;
mod util;
mod world;
mod worldgen;
mod items;
mod serialization;
mod ui;
mod net;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(UtilPlugin)
        .add_plugins(serialization::SerializationPlugin)
        .add_plugins(LevelPlugin)
        .add_plugins(MesherPlugin)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ChunkLoaderPlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(ControllersPlugin)
        .add_plugins(ActorPlugin)
        .add_plugins(ItemsPlugin)
        .add_plugins(ui::UIPlugin)
        .add_plugins(net::NetPlugin)
        .insert_resource(AmbientLight {
            brightness: 0.3,
            ..default()
        })
        .run();
}