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

use std::{env, net::Ipv4Addr};

use actors::ActorPlugin;
use bevy::prelude::*;
use bevy_hanabi::HanabiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use chunk_loading::{ChunkLoader, ChunkLoaderPlugin};
use controllers::ControllersPlugin;
use items::ItemsPlugin;

use mesher::MesherPlugin;
use net::client::ClientConfig;
use physics::PhysicsPlugin;
use util::plugin::UtilPlugin;
use world::*;
use worldgen::WorldGenPlugin;

use crate::net::{server::ServerConfig, NetworkType};

mod actors;
mod chunk_loading;
mod controllers;
mod gameplay;
mod items;
mod mesher;
mod net;
mod physics;
mod serialization;
mod ui;
mod util;
mod world;
mod worldgen;

fn main() {
    //todo - this should be in GUI
    let args: Vec<String> = env::args().collect();
    let mut server_port = None;
    let mut client_connection_ip = None;
    println!("ARGS: {:?}", args);
    if args.len() == 3 && args[1] == "host" {
        server_port = Some(args[2].parse::<u16>().unwrap());
        println!("Need to start server on port {}", server_port.unwrap());
    }
    if args.len() == 4 && args[1] == "join" {
        client_connection_ip = Some((
            args[2].parse::<std::net::IpAddr>().unwrap(),
            args[3].parse::<u16>().unwrap(),
        ));
        println!("Need to connect client to {:?}", client_connection_ip);
    }

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Echoes of the Outsider".to_string(),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(HanabiPlugin)
    .add_plugins(WorldInspectorPlugin::new())
    .add_plugins((
        UtilPlugin,
        serialization::SerializationPlugin,
        LevelPlugin,
        MesherPlugin,
        WorldGenPlugin,
        ChunkLoaderPlugin,
        PhysicsPlugin,
        ControllersPlugin,
        ActorPlugin,
        ItemsPlugin,
        ui::UIPlugin,
        net::NetPlugin,
        util::bevy_utils::BevyUtilsPlugin,
        gameplay::GameplayPlugin
    ))
    .insert_resource(AmbientLight {
        brightness: 0.3,
        ..default()
    });

    if let Some(port) = server_port {
        app.add_systems(
            Startup,
            move |mut commands: Commands, mut next_state: ResMut<NextState<NetworkType>>| {
                info!("Creating server config on port {}", port);
                commands.insert_resource(ServerConfig {
                    bind_addr: std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    bind_port: port,
                });
                next_state.set(NetworkType::Server);
            },
        );
    } else if let Some((ip, port)) = client_connection_ip {
        app.add_systems(
            Startup,
            move |mut commands: Commands, mut next_state: ResMut<NextState<NetworkType>>| {
                info!("Creating client config, connecting to {}:{}", ip, port);
                commands.insert_resource(ClientConfig {
                    server_ip: ip,
                    server_port: port,
                    local_ip: std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    local_port: 0,
                });
                next_state.set(NetworkType::Client);
            },
        );
    } else {
        app.add_systems(Startup, |mut next_state: ResMut<NextState<NetworkType>>| {
            next_state.set(NetworkType::Singleplayer);
        });
    }

    app.run();
}
