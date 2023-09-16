use crate::{physics::PhysicsObjectBundle, util::SendEventCommand};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{CombatInfo, CombatantBundle, ActorResources, ActorName};

#[derive(Resource)]
pub struct CoinResources {
    pub scene: Handle<Scene>,
}

#[derive(Component, Default)]
pub struct Coin;

#[derive(Component)]
pub struct CoinScene;

#[derive(Event)]
pub struct SpawnCoinEvent {
    pub location: GlobalTransform,
}

pub struct CoinPlugin;

impl Plugin for CoinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(Update, spawn_coin)
            .add_event::<SpawnCoinEvent>();
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("coin"),
        Box::new(|commands, tf| {
            commands.add(SendEventCommand(SpawnCoinEvent {
                location: tf
            }))
        }),
    );
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
) {
    for spawn in spawn_requests.iter() {
        commands.spawn((
            SceneBundle {
                scene: res.scene.clone_weak(),
                transform: spawn.location.compute_transform().with_scale(Vec3::new(0.5,0.5,0.5)),
                ..default()
            },
            Name::new("coin"),
            CombatantBundle {
                combat_info: CombatInfo {
                    ..CombatInfo::new(1.0, 0.0)
                },
                ..default()
            },
            PhysicsObjectBundle {
                rigidbody: RigidBody::Dynamic,
                collider: Collider::cylinder(0.0625, 0.175),
                locked_axes: LockedAxes::empty(),
                ..default()
            },
            Coin { ..default() },
            //no UninitializedActor b/c we don't have to do any setup
        ));
    }
}
