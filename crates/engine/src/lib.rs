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
pub mod controllers;
pub mod effects;
pub mod items;
pub mod state;

use ::util;
use bevy::prelude::*;

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            util::plugin::UtilPlugin,
            controllers::ControllersPlugin,
            actors::ActorPlugin,
            items::ItemsPlugin,
            effects::EffectsPlugin,
            camera::CameraPlugin,
            state::GameStatePlugin,
        ));
    }
}
