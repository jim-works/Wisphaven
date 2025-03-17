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
//lints created using dylint will give a warning
#![allow(unknown_lints)]

use std::env;

use bevy::{prelude::*, window::WindowResolution};
use bevy_hanabi::HanabiPlugin;
use interfaces::scheduling::{DebugUIState, GameState, NetworkType};

fn main() {
    //todo - this should be in GUI
    //todo - do better parsing
    let args: Vec<String> = env::args().collect();
    let mut server_port = None;
    const DEFAULT_SERVER_PORT: u16 = 15155;
    let mut skip_menu = false;
    let mut client_connection_string = None;
    println!("ARGS: {:?}", args);
    if args.len() == 2 && args[1] == "skip-menu" {
        skip_menu = true;
    }
    if args.len() == 2 && args[1] == "set-cwd" {
        // for debugging
        print!("SETTING CWD");
        env::set_current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("Failed to set CWD");
    }
    if args.len() == 3 && args[1] == "host" {
        server_port = Some(args[2].parse::<u16>().unwrap());
        println!("Need to start server on port {}", server_port.unwrap());
    }
    if args.len() == 3 && args[1] == "join" {
        client_connection_string = Some(args[2].clone());
        println!("Need to connect client to {:?}", args[2]);
    }
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Wisphaven".to_string(),
                    resolution: WindowResolution::new(1600.0, 900.0),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(HanabiPlugin)
    .add_plugins(
        bevy_inspector_egui::quick::WorldInspectorPlugin::default()
            .run_if(in_state(DebugUIState::Shown)),
    )
    .add_plugins((
        (
            interfaces::InterfacesPlugin,
            engine::EnginePlugin,
            ui::UIPlugin,
            ::actors::ActorsPlugin,
            waves::GameplayPlugin,
            ::items::ItemsPlugin,
            crafting::RecipePlugin,
            blocks::BlocksPlugin,
            net::NetPlugin,
            serialization::SerializationPlugin,
            world::LevelPlugin,
            debug::DebugUIPlugin,
            physics::PhysicsPlugin,
        ),
        citizens::CitizensPlugin,
        // new internal crates go here
    ));

    if server_port.is_some() {
        net::config::setup(&mut app, NetworkType::Host, server_port, None);
        app.add_systems(
            Startup,
            |mut next_game_state: ResMut<NextState<GameState>>| {
                next_game_state.set(GameState::Game);
            },
        );
    } else if client_connection_string.is_some() {
        net::config::setup(
            &mut app,
            NetworkType::Client,
            None,
            client_connection_string,
        );
        app.add_systems(
            Startup,
            |mut next_game_state: ResMut<NextState<GameState>>| {
                next_game_state.set(GameState::Game);
            },
        );
    } else {
        // start in host mode so we can both host and join games later
        net::config::setup(&mut app, NetworkType::Host, Some(DEFAULT_SERVER_PORT), None);
        app.add_systems(
            Startup,
            move |mut next_state: ResMut<NextState<NetworkType>>,
                  mut next_game_state: ResMut<NextState<GameState>>| {
                next_state.set(NetworkType::Host);
                next_game_state.set(if skip_menu {
                    GameState::Game
                } else {
                    GameState::Menu
                });
            },
        );
    }
    app.run();
}
