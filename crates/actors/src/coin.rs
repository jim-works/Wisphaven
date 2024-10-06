use std::time::Duration;

use bevy::prelude::*;
use engine::{
    all_teams_event, all_teams_system,
    physics::{collision::Aabb, movement::Velocity, PhysicsBundle},
};

use engine::actors::{
    projectile::{Projectile, ProjectileBundle},
    team::*,
    CombatantBundle, Damage,
};

#[derive(Resource)]
pub struct CoinResources {
    pub scene: Handle<Scene>,
}

#[derive(Component)]
pub struct CoinScene;

#[derive(Event)]
pub struct SpawnCoinEvent<T: Team> {
    pub location: Transform,
    pub velocity: Velocity,
    pub combat: CombatantBundle<T>,
    pub owner: Entity,
    pub damage: Damage,
}

pub struct CoinPlugin;

impl Plugin for CoinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(Update, all_teams_system!(spawn_coin));
        all_teams_event!(app, SpawnCoinEvent);
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
    _children_query: Query<&Children>,
    time: Res<Time>,
) {
    const LIFETIME: Duration = Duration::from_secs(10);
    let curr_time = time.elapsed();
    for spawn in spawn_requests.read() {
        commands.spawn((
            SceneBundle {
                scene: res.scene.clone_weak(),
                transform: spawn.location.with_scale(Vec3::new(0.5, 0.5, 0.5)),
                ..default()
            },
            Name::new("coin"),
            spawn.combat.clone(),
            PhysicsBundle {
                collider: Aabb::centered(Vec3::new(0.125, 0.0625, 0.125)),
                velocity: spawn.velocity,
                ..default()
            },
            ProjectileBundle::new(Projectile {
                owner: spawn.owner,
                knockback_mult: 1.0,
                terrain_damage: 0.5,
                despawn_time: curr_time + LIFETIME,
                damage: spawn.damage,
                despawn_on_hit: true,
                on_hit_or_despawn: None,
            }),
            //no UninitializedActor b/c we don't have to do any setup
        ));
    }
}
