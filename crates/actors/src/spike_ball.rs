use std::time::Duration;

use bevy::prelude::*;
use engine::{
    all_teams_function, all_teams_system,
    physics::{collision::Aabb, PhysicsBundle},
};

use engine::actors::{
    projectile::{Projectile, ProjectileBundle},
    team::*,
};

use crate::spawning::{BuildProjectileRegistry, DefaultSpawnArgs, ProjectileSpawnArgs};

#[derive(Resource)]
pub struct SpikeBallResources {
    pub scene: Handle<Scene>,
}

#[derive(Component)]
pub struct SpikeBallScene;

pub struct SpikeBallPlugin;

impl Plugin for SpikeBallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(Update, all_teams_system!(spawn_spike_ball));
        all_teams_function!(app, add_event, SpawnSpikeBallEvent);
        all_teams_function!(
            app,
            add_projectile,
            SpawnSpikeBallEvent,
            "spike_ball".to_string()
        );
    }
}

#[derive(Event)]
pub struct SpawnSpikeBallEvent<T: Team> {
    pub projectile_args: ProjectileSpawnArgs<T>,
    pub default_args: DefaultSpawnArgs,
}

impl<T: Team> From<(DefaultSpawnArgs, ProjectileSpawnArgs<T>)> for SpawnSpikeBallEvent<T> {
    fn from(value: (DefaultSpawnArgs, ProjectileSpawnArgs<T>)) -> Self {
        Self {
            projectile_args: value.1,
            default_args: value.0,
        }
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SpikeBallResources {
        scene: assets.load("actors/spike_ball/spike_ball.glb#Scene0"),
    });
}

pub fn spawn_spike_ball<T: Team>(
    mut commands: Commands,
    res: Res<SpikeBallResources>,
    mut spawn_requests: EventReader<SpawnSpikeBallEvent<T>>,
    time: Res<Time>,
) {
    const LIFETIME: Duration = Duration::from_secs(10);
    let curr_time = time.elapsed();
    for SpawnSpikeBallEvent {
        projectile_args,
        default_args,
    } in spawn_requests.read()
    {
        commands.spawn((
            SceneBundle {
                scene: res.scene.clone_weak(),
                transform: default_args.transform,
                ..default()
            },
            Name::new("spike_ball"),
            projectile_args.combat.clone(),
            PhysicsBundle {
                collider: Aabb::centered(Vec3::ONE),
                velocity: projectile_args.velocity,
                ..default()
            },
            ProjectileBundle::new(Projectile {
                owner: projectile_args.owner,
                knockback_mult: 1.0,
                terrain_damage: 0.5,
                despawn_time: curr_time + LIFETIME,
                damage: projectile_args.damage,
                despawn_on_hit: true,
                on_hit_or_despawn: None,
            }),
            //no UninitializedActor b/c we don't have to do any setup
        ));
    }
}
