use crate::{
    ui::healthbar::spawn_billboard_healthbar, util::SendEventCommand, physics::{PhysicsBundle, collision::Aabb}
};
use bevy::prelude::*;

use super::{CombatInfo, CombatantBundle, ActorResources, ActorName};

#[derive(Resource)]
pub struct WorldAnchorResources {
    pub scene: Handle<Scene>,
}

//can use presence of this resource to easily detect if we're ready to spawn waves
#[derive(Component, Resource, Default, Clone, Copy)]
pub struct WorldAnchor;

#[derive(Component)]
pub struct WorldAnchorScene;

#[derive(Event)]
pub struct SpawnWorldAnchorEvent {
    pub location: Transform,
}

pub struct WorldAnchorPlugin;

impl Plugin for WorldAnchorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(Update, spawn_world_anchor)
            .add_event::<SpawnWorldAnchorEvent>();
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("world_anchor"),
        Box::new(|commands, tf| {
            commands.add(SendEventCommand(SpawnWorldAnchorEvent {
                location: tf
            }))
        }),
    );
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(WorldAnchorResources {
        scene: assets.load("anchor/anchor.gltf#Scene0"),
    });
}

fn trigger_spawning(mut writer: EventWriter<SpawnWorldAnchorEvent>) {
    writer.send(SpawnWorldAnchorEvent {
        location: Transform::from_xyz(0.0, 0.0, 0.0),
    });
}

pub fn spawn_world_anchor(
    mut commands: Commands,
    res: Res<WorldAnchorResources>,
    mut spawn_requests: EventReader<SpawnWorldAnchorEvent>,
    _children_query: Query<&Children>,
) {
    for spawn in spawn_requests.read() {
        let id = commands
            .spawn((
                SceneBundle {
                    scene: res.scene.clone_weak(),
                    transform: spawn.location.with_scale(Vec3::new(2.0,2.0,2.0)),
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
                PhysicsBundle {
                    //center of anchor is at bottom of model, so spawn the collision box offset
                    collider: Aabb::new(Vec3::new(2.0,2.0,2.0), Vec3::new(-1.0,0.0,-1.0)),
                    ..default()
                },
                WorldAnchor,
                //no UninitializedActor b/c we don't have to do any setup
            ))
            .id();
        commands.insert_resource(WorldAnchor);
        //add healthbar
        spawn_billboard_healthbar(
            &mut commands,
            id,
            Vec3::new(0.0, 2.0, 0.0),
        );
    }
}
