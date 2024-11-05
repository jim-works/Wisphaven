use std::time::Duration;

use bevy::prelude::*;
use engine::{
    all_teams_function, all_teams_system,
    physics::{
        collision::Aabb,
        movement::{Drag, Restitution},
        PhysicsBundle,
    },
};

use engine::actors::{
    projectile::{Projectile, ProjectileBundle},
    team::*,
};

use crate::spawning::{BuildProjectileRegistry, DefaultSpawnArgs, ProjectileSpawnArgs};

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

fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SpikeBallResources {
        scene: assets.load("actors/spike_ball/spike_ball.glb#Scene0"),
        spawn_audio: assets.load("sounds/spike_ball.ogg"),
    });
}

fn spawn_spike_ball<T: Team>(
    mut commands: Commands,
    res: Res<SpikeBallResources>,
    mut spawn_requests: EventReader<SpawnSpikeBallEvent<T>>,
    time: Res<Time>,
) {
    const LIFETIME: Duration = Duration::from_secs(5);
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
                collider: Aabb::centered(1.5 * Vec3::ONE),
                velocity: projectile_args.velocity,
                restitution: Restitution(1.),
                drag: Drag(0.),
                ..default()
            },
            ProjectileBundle::new(Projectile {
                owner: projectile_args.owner,
                knockback_mult: 1.0 * projectile_args.knockback_mult,
                terrain_damage: 0.5 * projectile_args.terrain_damage_mult,
                despawn_time: curr_time + LIFETIME.mul_f32(projectile_args.lifetime_mult),
                damage: projectile_args.damage,
                hit_behavior: engine::actors::projectile::ProjecileHitBehavior::None,
                on_hit: None,
            }),
            AudioBundle {
                source: res.spawn_audio.clone(),
                settings: PlaybackSettings::ONCE,
            },
        ));
    }
}
