//disable console window from popping up on windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//have to enable this because it's a nursery feature
#![warn(clippy::disallowed_types)]
//bevy system signatures often violate these rules
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//TODO: remove this before release. annoying as balls during development
#![allow(dead_code)]

use std::{env, net::Ipv4Addr};

use actors::ActorPlugin;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use chunk_loading::{ChunkLoader, ChunkLoaderPlugin};
use controllers::ControllersPlugin;
use items::ItemsPlugin;

use mesher::MesherPlugin;
use net::{client::StartClientEvent, server::StartServerEvent};
use physics::PhysicsPlugin;
use util::plugin::UtilPlugin;
use world::*;
use worldgen::WorldGenPlugin;

use crate::net::NetworkType;

mod actors;
mod chunk_loading;
mod controllers;
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
    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
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
        });

    if let Some(port) = server_port {
        app.add_systems(Startup, move |mut writer: EventWriter<StartServerEvent>, mut commands: Commands| {
            info!("Sending start server event on port {}", port);
            writer.send(StartServerEvent {
                bind_addr: std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                bind_port: port,
            });
            commands.insert_resource(NetworkType::Server);
        });
    } else if let Some((ip, port)) = client_connection_ip {
        app.add_systems(Startup, move |mut writer: EventWriter<StartClientEvent>, mut commands: Commands| {
            info!("Sending start client event, connecting to {}:{}", ip, port);
            writer.send(StartClientEvent {
                server_ip: ip,
                server_port: port,
                local_ip: std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                local_port: 0,
            });
            commands.insert_resource(NetworkType::Client);
        });
    } else {
        app.add_systems(Startup, |mut commands: Commands| {
            commands.insert_resource(NetworkType::Singleplayer);
        });
    }

    app.run();
}
