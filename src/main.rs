//disable console window from popping up on windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]



use actors::ActorPlugin;
use bevy::prelude::*;
use bevy_atmosphere::prelude::*;
use bevy_mod_billboard::prelude::*;
use bevy_fly_camera::FlyCameraPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use chunk_loading::{ChunkLoader, ChunkLoaderPlugin};
use controllers::ControllersPlugin;
use items::ItemsPlugin;

use mesher::MesherPlugin;
use physics::PhysicsPlugin;
use world::{*, events::CreateLevelEvent};
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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(BillboardPlugin)
        .add_plugin(AtmospherePlugin)
        .add_plugin(LevelPlugin)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(MesherPlugin)
        .add_plugin(WorldGenPlugin)
        .add_plugin(ChunkLoaderPlugin)
        .add_plugin(PhysicsPlugin)
        .add_plugin(ControllersPlugin)
        .add_plugin(ActorPlugin)
        .add_plugin(ItemsPlugin)
        .add_plugin(serialization::SerializationPlugin)
        .add_plugin(ui::UIPlugin)
        .insert_resource(settings::Settings::default())
        .insert_resource(AmbientLight {
            brightness: 0.3,
            ..default()
        })
        .add_startup_system(init)
        .run();
}

fn init(mut writer: EventWriter<CreateLevelEvent>) {
    writer.send(CreateLevelEvent { name: "level", seed: 8008135 });
    info!("Sent create level event!");
}