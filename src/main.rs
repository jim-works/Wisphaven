use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use chunk_loading::{ChunkLoader, ChunkLoaderPlugin};
use mesher::MesherPlugin;
use world::BlockType;
use world::*;
use world::chunk::ChunkType;
use worldgen::WorldGenPlugin;
use crate::{mesher::ChunkNeedsMesh, world::chunk::ChunkCoord};

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
        .insert_resource(Level::new())
        .add_startup_system(init)
        .add_system(animate_light_direction)
        //.add_system(remove_block)
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
            ..default()
        },
        ChunkLoader {
            radius: 8
        },
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: false,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        ..default()
    });
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() * 0.5);
    }
}

fn remove_block(
    time: Res<Time>,
    mut commands: Commands,
    level: Res<Level>,
    mut query: Query<(Entity, &ChunkCoord), Without<ChunkNeedsMesh>>,
) {
    if (time.elapsed_seconds() - time.delta_seconds()).floor() == time.elapsed_seconds().floor() {
        return;
    }
    let idx = (time.elapsed_seconds() * 2.0) as usize;
    for (entity, coord) in &mut query {
        if let Some(mut ctype) = level.chunks.get_mut(coord) {
            let v = ctype.value_mut();
            if let ChunkType::Full(chunk) = v {
                chunk[idx] = BlockType::Empty;
                commands.entity(entity).insert(ChunkNeedsMesh {});
            }
        }
    }
}
