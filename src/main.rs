use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use chunk_loading::{ChunkLoader, ChunkLoaderPlugin};
use mesher::MesherPlugin;
use world::*;
use worldgen::WorldGenPlugin;

mod mesher;
mod util;
mod world;
mod worldgen;
mod chunk_loading;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(LevelPlugin)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(MesherPlugin)
        .add_plugin(WorldGenPlugin)
        .add_plugin(ChunkLoaderPlugin)
        .insert_resource(Level::new(5))
        .insert_resource(AmbientLight {
            brightness: 0.3,
            ..default()
        })
        .add_startup_system(init)
        .run();

}

fn init(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            projection: Projection::Perspective(PerspectiveProjection {
                fov: PI / 2.,
                ..default()
            }),
            ..default()
        },
        FlyCamera {
            accel: 10.0,
            max_speed: 2.0,
            sensitivity: 10.0,
            ..default()
        },
        ChunkLoader {
            radius: 4,
            lod_levels: 16,
        },
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-100.0, -10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
