use std::time::Duration;

use bevy::prelude::*;
use interfaces::scheduling::LevelSystemSet;
use physics::collision::Aabb;

use serde::Deserialize;

use crate::spawning::{BuildProjectileRegistry, ProjectileName, SpawnProjectileEvent};

#[derive(Resource)]
pub struct CoinResources {
    pub scene: Handle<Scene>,
}

#[derive(Component)]
pub struct CoinScene;

pub struct CoinPlugin;

impl Plugin for CoinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(FixedUpdate, spawn_coin.in_set(LevelSystemSet::PostTick))
            .add_projectile::<SpawnCoin>(ProjectileName::core("coin"));
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SpawnCoin;

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(CoinResources {
        scene: assets.load("coin/coin.gltf#Scene0"),
    });
}

pub fn spawn_coin(
    mut commands: Commands,
    res: Res<CoinResources>,
    mut spawn_requests: EventReader<SpawnProjectileEvent<SpawnCoin>>,
    time: Res<Time>,
) {
    let curr_time = time.elapsed();
    for req in spawn_requests.read() {
        let mut ec = commands.spawn_empty();
        req.args.spawn(&mut ec, curr_time);
        ec.insert((
            Name::new("coin"),
            Aabb::centered(Vec3::new(0.125, 0.0625, 0.125)),
            SceneRoot(res.scene.clone_weak()),
        ));
    }
}
