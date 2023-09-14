use crate::{
    physics::PhysicsObjectBundle,
    ui::healthbar::{spawn_billboard_healthbar, HealthbarResources},
    world::LevelLoadState,
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{CombatInfo, CombatantBundle};

#[derive(Resource)]
pub struct WorldAnchorResources {
    pub scene: Handle<Scene>,
}

#[derive(Component, Default)]
pub struct WorldAnchor;

#[derive(Component)]
pub struct WorldAnchorScene;

#[derive(Event)]
pub struct SpawnWorldAnchorEvent {
    pub location: GlobalTransform,
}

pub struct WorldAnchorPlugin;

impl Plugin for WorldAnchorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_systems(Update, spawn_world_anchor)
            .add_event::<SpawnWorldAnchorEvent>();
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(WorldAnchorResources {
        scene: assets.load("anchor/anchor.gltf#Scene0"),
    });
}

fn trigger_spawning(mut writer: EventWriter<SpawnWorldAnchorEvent>) {
    writer.send(SpawnWorldAnchorEvent {
        location: GlobalTransform::from_xyz(0.0, 0.0, 0.0),
    });
}

pub fn spawn_world_anchor(
    mut commands: Commands,
    res: Res<WorldAnchorResources>,
    mut spawn_requests: EventReader<SpawnWorldAnchorEvent>,
    healthbar_resources: Res<HealthbarResources>,
    _children_query: Query<&Children>,
) {
    for spawn in spawn_requests.iter() {
        let id = commands
            .spawn((
                SceneBundle {
                    scene: res.scene.clone_weak(),
                    transform: spawn.location.compute_transform(),
                    ..default()
                },
                Name::new("world anchor"),
                CombatantBundle {
                    combat_info: CombatInfo {
                        knockback_multiplier: 0.0,
                        ..CombatInfo::new(10.0, 0.0)
                    },
                    ..default()
                },
                PhysicsObjectBundle {
                    rigidbody: RigidBody::Fixed,
                    collider: Collider::cuboid(0.5, 0.5, 0.5),
                    ..default()
                },
                WorldAnchor { ..default() },
                //no UninitializedActor b/c we don't have to do any setup
            ))
            .id();
        //add healthbar
        spawn_billboard_healthbar(
            &mut commands,
            &healthbar_resources,
            id,
            Vec3::new(0.0, 2.0, 0.0),
        );
    }
}
