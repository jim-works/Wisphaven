use bevy::prelude::*;

use crate::{
    physics::{PhysicsBundle, collision::Aabb, movement::GravityMult},
    util::{plugin::SmoothLookTo, SendEventCommand},
    world::LevelLoadState,
};

use super::{
    ActorName, ActorResources, CombatInfo, CombatantBundle, Idler,
};

#[derive(Resource)]
pub struct GhostResources {
    pub scene: Handle<Scene>,
}

#[derive(Component, Default)]
pub struct Ghost;

#[derive(Event)]
pub struct SpawnGhostEvent {
    pub location: GlobalTransform,
}

pub struct GhostPlugin;

impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_systems(Update, spawn_ghost)
            .add_event::<SpawnGhostEvent>();
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(GhostResources {
        scene: assets.load("ghost/ghost.gltf#Scene0"),
    });
}

fn trigger_spawning(mut writer: EventWriter<SpawnGhostEvent>) {
    for i in 0..5 {
        writer.send(SpawnGhostEvent {
            location: GlobalTransform::from_xyz((i%5) as f32 * -5.0, (i/5) as f32 * 5.0 + 50.0, 0.0),
        });
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("ghost"),
        Box::new(|commands, tf| commands.add(SendEventCommand(SpawnGhostEvent { location: tf }))),
    );
}

fn spawn_ghost(
    mut commands: Commands,
    res: Res<GhostResources>,
    mut spawn_requests: EventReader<SpawnGhostEvent>,
) {
    for spawn in spawn_requests.read() {
        commands.spawn((
            SceneBundle {
                scene: res.scene.clone_weak(),
                transform: spawn.location.compute_transform(),
                ..default()
            },
            Name::new("ghost"),
            CombatantBundle {
                combat_info: CombatInfo {
                    knockback_multiplier: 2.0,
                    ..CombatInfo::new(10.0, 0.0)
                },
                ..default()
            },
            PhysicsBundle {
                collider: Aabb::centered(Vec3::splat(0.5)),
                gravity: GravityMult(0.1),
                ..default()
            },
            Ghost,
            Idler::default(),
            SmoothLookTo::new(0.5),
            bevy::pbr::CubemapVisibleEntities::default(),
            bevy::render::primitives::CubemapFrusta::default(),
        ));
    }
}
