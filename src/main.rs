use std::f32::consts::PI;

use actors::{Jump, MoveSpeed, Player, LocalPlayer};
use bevy::prelude::*;
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use bevy_rapier3d::prelude::*;
use chunk_loading::{ChunkLoader, ChunkLoaderPlugin};
use controllers::{ControllersPlugin, RotateWithMouse, FollowPlayer, ControllableBundle, PlayerActionOrigin};
use leafwing_input_manager::InputManagerBundle;
use mesher::MesherPlugin;
use physics::{PhysicsPlugin, ACTOR_GROUP, PLAYER_GROUP, JUMPABLE_GROUP, PhysicsObjectBundle};
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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(LevelPlugin)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(MesherPlugin)
        .add_plugin(WorldGenPlugin)
        .add_plugin(ChunkLoaderPlugin)
        .add_plugin(PhysicsPlugin)
        .add_plugin(ControllersPlugin)
        .insert_resource(Level::new(5))
        .insert_resource(AmbientLight {
            brightness: 0.3,
            ..default()
        })
        .add_startup_system(init)
        .run();
}

fn init(mut commands: Commands) {
    commands
        .spawn((
            
            Player {},
            LocalPlayer {},
            RotateWithMouse {
                lock_pitch: true,
                ..default()
            },
            TransformBundle::from_transform(Transform::from_xyz(-2.0, -25.0, 5.0)),
            ControllableBundle {
                physics: PhysicsObjectBundle {
                    collision_groups: CollisionGroups::new(
                    Group::from_bits_truncate(PLAYER_GROUP | ACTOR_GROUP),
                    Group::all(),
                    ),
                    ..default()
                },
                ..default()
            },
            Jump::default(),
            ChunkLoader {
                radius: 4,
                lod_levels: 3,
            },
            InputManagerBundle {
                input_map: controllers::get_input_map(),
                ..default()
            },
        ));
    commands.spawn((Camera3dBundle {
                transform: Transform::from_xyz(0.0,1.5,0.0),
                projection: Projection::Perspective(PerspectiveProjection {
                    fov: PI / 2.,
                    ..default()
                }),
                ..default()
            },
            RotateWithMouse {
                pitch_bound: PI * 0.49,
                lock_yaw: true,
                ..default()
            },
            FollowPlayer{},
            PlayerActionOrigin{},
            InputManagerBundle {
                input_map: controllers::get_input_map(),
                ..default()
            },
            ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
