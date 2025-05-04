use bevy::prelude::*;
use engine::actors::projectile::Projectile;
use engine::items::ItemName;
use engine::items::loot::{ItemLootTable, ItemLootTableDrop};
use engine::{self, items::ItemResources};
use interfaces::scheduling::LevelSystemSet;
use physics::{
    collision::Aabb,
    movement::{Drag, Restitution},
};
use serde::Deserialize;

use crate::spawning::{BuildProjectileRegistry, ProjectileName, SpawnProjectileEvent};

#[derive(Resource)]
struct RockResources {
    rock_scene: Handle<Scene>,
    hammer_scene: Handle<Scene>,
    spawn_audio: Handle<AudioSource>,
}

pub struct RockPlugin;

impl Plugin for RockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(
                FixedUpdate,
                (spawn_rock, spawn_rock_hammer).in_set(LevelSystemSet::PostTick),
            )
            .add_projectile::<SpawnRock>(ProjectileName::core("rock"))
            .add_projectile::<SpawnRockHammer>(ProjectileName::core("rock_hammer"));
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SpawnRock;

#[derive(Debug, Default, Deserialize)]
pub struct SpawnRockHammer;

fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(RockResources {
        rock_scene: assets.load("actors/rock/rock.glb#Scene0"),
        hammer_scene: assets.load("actors/rock/rock_hammer.glb#Scene0"),
        spawn_audio: assets.load("sounds/spike_ball.ogg"),
    });
}

fn spawn_rock(
    mut commands: Commands,
    res: Res<RockResources>,
    mut spawn_requests: EventReader<SpawnProjectileEvent<SpawnRock>>,
    time: Res<Time>,
) {
    let curr_time = time.elapsed();
    for SpawnProjectileEvent::<SpawnRock> { args, event: _ } in spawn_requests.read() {
        let mut ec = commands.spawn_empty();
        let proj = args.spawn(&mut ec, curr_time);
        ec.insert((
            Name::new("rock"),
            SceneRoot(res.rock_scene.clone_weak()),
            Aabb::centered(0.9 * Vec3::ONE),
            Drag(0.),
            AudioPlayer(res.spawn_audio.clone()),
            ItemLootTable {
                drops: vec![ItemLootTableDrop {
                    item: ItemName::core("rock"),
                    drop_chance: 0.75,
                    drop_count_range: (1, 1),
                }],
            },
            PlaybackSettings::ONCE,
            Projectile {
                hit_behavior: engine::actors::projectile::ProjecileHitBehavior::Despawn,
                ..proj
            },
        ));
    }
}

fn spawn_rock_hammer(
    mut commands: Commands,
    res: Res<RockResources>,
    mut spawn_requests: EventReader<SpawnProjectileEvent<SpawnRockHammer>>,
    time: Res<Time>,
) {
    let curr_time = time.elapsed();
    for SpawnProjectileEvent::<SpawnRockHammer> { args, event: _ } in spawn_requests.read() {
        let mut ec = commands.spawn_empty();
        let proj = args.spawn(&mut ec, curr_time);
        ec.insert((
            Name::new("rock_hammer"),
            SceneRoot(res.hammer_scene.clone_weak()),
            Aabb::centered(Vec3::new(0.25, 1., 0.25)),
            Drag(0.),
            AudioPlayer(res.spawn_audio.clone()),
            ItemLootTable {
                drops: vec![ItemLootTableDrop {
                    item: ItemName::core("rock_hammer"),
                    drop_chance: 1.0,
                    drop_count_range: (1, 1),
                }],
            },
            PlaybackSettings::ONCE,
            Projectile {
                hit_behavior: engine::actors::projectile::ProjecileHitBehavior::Despawn,
                ..proj
            },
        ));
    }
}
