use std::time::Duration;

use bevy::prelude::*;
use engine::{all_teams_function, all_teams_system};
use interfaces::scheduling::LevelLoadState;
use physics::{collision::Aabb, PhysicsBundle};

use engine::actors::{
    projectile::{Projectile, ProjectileBundle},
    team::*,
};

use crate::spawning::{BuildProjectileRegistry, DefaultSpawnArgs, ProjectileSpawnArgs};

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
            .add_systems(Update, all_teams_system!(spawn_coin));
        all_teams_function!(app, add_event, SpawnCoinEvent);
        all_teams_function!(app, add_projectile, SpawnCoinEvent, "coin".to_string());
    }
}

#[derive(Event)]
pub struct SpawnCoinEvent<T: Team> {
    pub projectile_args: ProjectileSpawnArgs<T>,
    pub default_args: DefaultSpawnArgs,
}

impl<T: Team> From<(DefaultSpawnArgs, ProjectileSpawnArgs<T>)> for SpawnCoinEvent<T> {
    fn from(value: (DefaultSpawnArgs, ProjectileSpawnArgs<T>)) -> Self {
        Self {
            projectile_args: value.1,
            default_args: value.0,
        }
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(CoinResources {
        scene: assets.load("coin/coin.gltf#Scene0"),
    });
}

pub fn spawn_coin<T: Team>(
    mut commands: Commands,
    res: Res<CoinResources>,
    mut spawn_requests: EventReader<SpawnCoinEvent<T>>,
    time: Res<Time>,
) {
    const LIFETIME: Duration = Duration::from_secs(10);
    let curr_time = time.elapsed();
    for SpawnCoinEvent {
        projectile_args,
        default_args,
    } in spawn_requests.read()
    {
        commands.spawn((
            StateScoped(LevelLoadState::Loaded),
            SceneRoot(res.scene.clone_weak()),
            default_args.transform,
            Name::new("coin"),
            projectile_args.combat.clone(),
            PhysicsBundle {
                collider: Aabb::centered(Vec3::new(0.125, 0.0625, 0.125)),
                velocity: projectile_args.velocity,
                ..default()
            },
            ProjectileBundle::new(Projectile {
                owner: projectile_args.owner,
                knockback_mult: 1.0,
                terrain_damage: 0.5,
                despawn_time: curr_time + LIFETIME,
                damage: projectile_args.damage,
                hit_behavior: engine::actors::projectile::ProjecileHitBehavior::Despawn,
                on_hit: None,
            }),
            //no UninitializedActor b/c we don't have to do any setup
        ));
    }
}
