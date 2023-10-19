use std::time::Duration;

use crate::physics::PhysicsObjectBundle;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{CombatantBundle, Damage, projectile::{ProjectileBundle, Projectile}};

#[derive(Resource)]
pub struct CoinResources {
    pub scene: Handle<Scene>,
}

#[derive(Component, Default)]
pub struct Coin {
    pub damage: Damage,
}

#[derive(Component)]
pub struct CoinScene;

#[derive(Event)]
pub struct SpawnCoinEvent {
    pub location: Transform,
    pub velocity: Vec3,
    pub combat: CombatantBundle,
    pub owner: Entity,
    pub damage: Damage,
}

pub struct CoinPlugin;

impl Plugin for CoinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(Update, spawn_coin)
            .add_event::<SpawnCoinEvent>();
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(CoinResources {
        scene: assets.load("coin/coin.gltf#Scene0"),
    });
}

pub fn spawn_coin(
    mut commands: Commands,
    res: Res<CoinResources>,
    mut spawn_requests: EventReader<SpawnCoinEvent>,
    _children_query: Query<&Children>,
    time: Res<Time>
) {
    const LIFETIME: Duration = Duration::from_secs(10);
    let curr_time = time.elapsed();
    for spawn in spawn_requests.iter() {
        commands.spawn((
            SceneBundle {
                scene: res.scene.clone_weak(),
                transform: spawn
                    .location
                    .with_scale(Vec3::new(0.5, 0.5, 0.5)),
                ..default()
            },
            Name::new("coin"),
            spawn.combat.clone(),
            PhysicsObjectBundle {
                rigidbody: RigidBody::Dynamic,
                collider: Collider::cylinder(0.0625, 0.175),
                locked_axes: LockedAxes::empty(),
                velocity: Velocity::linear(spawn.velocity),
                ..default()
            },
            Damping::default(),
            SolverGroups::new(Group::empty(), Group::empty()),
            Coin {
                damage: spawn.damage,
            },
            ProjectileBundle::new(Projectile {
                owner: spawn.owner,
                knockback_mult: 1.0,
                terrain_damage: 0.5,
                despawn_time: curr_time + LIFETIME,
                damage: Damage::default(),
                despawn_on_hit: true,
                on_hit_or_despawn: None,
            }),
            //no UninitializedActor b/c we don't have to do any setup
        ));
    }
}
