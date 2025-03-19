use bevy::prelude::*;
use engine;
use engine::actors::projectile::Projectile;
use interfaces::scheduling::LevelSystemSet;
use physics::{
    collision::Aabb,
    movement::{Drag, Restitution},
};
use serde::Deserialize;

use crate::spawning::{BuildProjectileRegistry, ProjectileName, SpawnProjectileEvent};

#[derive(Resource)]
struct SpikeBallResources {
    scene: Handle<Scene>,
    spawn_audio: Handle<AudioSource>,
}

#[derive(Component)]
pub struct SpikeBallScene;

pub struct SpikeBallPlugin;

impl Plugin for SpikeBallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(
                FixedUpdate,
                spawn_spike_ball.in_set(LevelSystemSet::PostTick),
            )
            .add_projectile::<SpawnSpikeBall>(ProjectileName::core("spike_ball"));
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SpawnSpikeBall {}

fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SpikeBallResources {
        scene: assets.load("actors/spike_ball/spike_ball.glb#Scene0"),
        spawn_audio: assets.load("sounds/spike_ball.ogg"),
    });
}

fn spawn_spike_ball(
    mut commands: Commands,
    res: Res<SpikeBallResources>,
    mut spawn_requests: EventReader<SpawnProjectileEvent<SpawnSpikeBall>>,
    time: Res<Time>,
) {
    let curr_time = time.elapsed();
    for SpawnProjectileEvent::<SpawnSpikeBall> { args, event: _ } in spawn_requests.read() {
        let mut ec = commands.spawn_empty();
        let proj = args.spawn(&mut ec, curr_time);
        ec.insert((
            Name::new("spike_ball"),
            SceneRoot(res.scene.clone_weak()),
            Aabb::centered(1.5 * Vec3::ONE),
            Restitution(1.),
            Drag(0.),
            AudioPlayer(res.spawn_audio.clone()),
            PlaybackSettings::ONCE,
            Projectile {
                hit_behavior: engine::actors::projectile::ProjecileHitBehavior::None,
                ..proj
            },
        ));
    }
}
