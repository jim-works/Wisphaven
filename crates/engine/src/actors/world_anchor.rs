use core::f32;

use crate::{
    chunk_loading::ChunkLoader,
    physics::{collision::Aabb, movement::Mass, PhysicsBundle},
    util::SendEventCommand,
    world::{settings::Settings, Level, LevelLoadState},
};
use bevy::prelude::*;

use super::{team::PlayerTeam, ActorName, ActorResources, Combatant, CombatantBundle};

#[derive(Resource)]
pub struct WorldAnchorResources {
    pub scene: Handle<Scene>,
}

//can use presence of this resource to easily detect if we're ready to spawn waves
#[derive(Component, Default, Clone, Copy)]
pub struct WorldAnchor;

#[derive(Resource, Clone, Copy)]
pub struct ActiveWorldAnchor(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct WorldAnchorHasSpawned;

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
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_systems(OnExit(LevelLoadState::Loaded), cleanup)
            .add_observer(on_world_anchor_destroyed)
            .add_event::<SpawnWorldAnchorEvent>();
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("world_anchor"),
        Box::new(|commands, tf| {
            commands.queue(SendEventCommand(SpawnWorldAnchorEvent { location: tf }))
        }),
    );
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(WorldAnchorResources {
        scene: assets.load("anchor/anchor.gltf#Scene0"),
    });
}

fn trigger_spawning(mut writer: EventWriter<SpawnWorldAnchorEvent>, level: Res<Level>) {
    writer.send(SpawnWorldAnchorEvent {
        location: Transform::from_translation(level.get_spawn_point()),
    });
}

fn cleanup(mut commands: Commands) {
    info!("cleanup called");
    commands.remove_resource::<ActiveWorldAnchor>();
    commands.remove_resource::<WorldAnchorHasSpawned>();
}

pub fn spawn_world_anchor(
    mut commands: Commands,
    res: Res<WorldAnchorResources>,
    mut spawn_requests: EventReader<SpawnWorldAnchorEvent>,
    settings: Res<Settings>,
    _children_query: Query<&Children>,
) {
    for spawn in spawn_requests.read() {
        let anchor = commands
            .spawn((
                StateScoped(LevelLoadState::Loaded),
                SceneRoot(res.scene.clone_weak()),
                spawn.location.with_scale(Vec3::new(2.0, 2.0, 2.0)),
                Name::new("world anchor"),
                CombatantBundle::<PlayerTeam> {
                    combatant: Combatant::new(10., 0.),
                    ..default()
                },
                PhysicsBundle {
                    //center of anchor is at bottom of model, so spawn the collision box offset
                    collider: Aabb::new(Vec3::new(2.0, 2.0, 2.0), Vec3::new(-1.0, 0.0, -1.0)),
                    mass: Mass(f32::INFINITY),
                    ..default()
                },
                WorldAnchor,
                ChunkLoader {
                    mesh: false,
                    ..settings.init_loader.clone()
                }, //no UninitializedActor b/c we don't have to do any setup
            ))
            .id();
        commands.insert_resource(ActiveWorldAnchor(anchor));
        commands.insert_resource(WorldAnchorHasSpawned);
    }
}

fn clear_resources(mut commands: Commands) {
    commands.remove_resource::<ActiveWorldAnchor>();
    commands.remove_resource::<WorldAnchorHasSpawned>();
}

fn on_world_anchor_destroyed(
    trigger: Trigger<OnRemove, WorldAnchor>,
    mut commands: Commands,
    mut active: ResMut<ActiveWorldAnchor>,
    query: Query<Entity, With<WorldAnchor>>,
) {
    let mut new_anchor = None;

    for entity in query.iter() {
        if entity != trigger.entity() {
            //there was another world anchor (somehow), promote it to be active
            new_anchor = Some(entity);
            break;
        }
    }
    if let Some(entity) = new_anchor {
        info!("new world anchor {:?}!!!!!!", entity);
        active.0 = entity;
    } else {
        info!("world anchor destroyed!!!!!!");
        commands.remove_resource::<ActiveWorldAnchor>();
    }
}
