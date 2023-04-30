use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use chunk::chunk::{BlockType, Chunk};
use mesher::MesherPlugin;

use crate::{chunk::chunk::{BLOCKS_PER_CHUNK, ChunkCoord}, mesher::mesher::ChunkNeedsMesh};

mod chunk;
mod mesher;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(MesherPlugin)
        .add_startup_system(init)
        .add_system(animate_light_direction)
        .add_system(remove_block)
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
        FlyCamera::default(),
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
    for x in 0..10 {
        for z in 0..10 {
        let mut chunk = Chunk::new(ChunkCoord::new(x,0,z));
        for i in 0..BLOCKS_PER_CHUNK {
            if i % 2 == 0 {
                chunk[i] = BlockType::Basic(0);
            }
        }

        commands.spawn((chunk, ChunkNeedsMesh {}));
        println!("Spawned chunk!");
    }
    }
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
    mut query: Query<(Entity, &mut Chunk), Without<ChunkNeedsMesh>>,
) {
    if (time.elapsed_seconds() - time.delta_seconds()).floor() == time.elapsed_seconds().floor() {
        return;
    }
    let idx = (time.elapsed_seconds() * 2.0) as usize;
    for (entity, mut chunk) in &mut query {
        chunk[idx] = BlockType::Empty;
        commands.entity(entity).insert(ChunkNeedsMesh {});
    }
}
