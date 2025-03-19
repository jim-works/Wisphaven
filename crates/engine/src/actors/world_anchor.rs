use core::f32;

use bevy::prelude::*;
use interfaces::scheduling::*;
use physics::{PhysicsBundle, collision::Aabb, movement::Mass};
use serde::Deserialize;
use world::{
    atmosphere::Calendar, chunk_loading::ChunkLoader, level::Level, settings::Settings,
    spawn_point::SpawnPoint,
};

use super::{
    ActorName, ActorResources, BuildActorRegistry, Combatant, CombatantBundle, DeathEvent,
    DeathInfo, SpawnActorEvent,
};

#[derive(Resource)]
pub struct WorldAnchorResources {
    pub scene: Handle<Scene>,
}

//can use presence of this resource to easily detect if we're ready to spawn waves
#[derive(Component, Default, Clone, Copy)]
pub struct WorldAnchor;

#[derive(Component)]
pub struct ActiveWorldAnchor;

#[derive(Default, Debug, Deserialize)]
pub struct SpawnWorldAnchor;

pub struct WorldAnchorPlugin;

impl Plugin for WorldAnchorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(Update, spawn_world_anchor)
            .add_systems(
                FixedUpdate,
                (active_on_day, set_spawn_on_add).in_set(LevelSystemSet::PostTick),
            )
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_observer(on_world_anchor_destroyed)
            .add_actor::<SpawnWorldAnchor>(ActorName::core("world_anchor"));
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry
        .add_dynamic::<SpawnWorldAnchor>(ActorName::core("world_anchor"));
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(WorldAnchorResources {
        scene: assets.load("anchor/anchor.gltf#Scene0"),
    });
}

fn trigger_spawning(
    mut writer: EventWriter<SpawnActorEvent<SpawnWorldAnchor>>,
    spawn_point: Res<SpawnPoint>,
    level: Res<Level>,
) {
    writer.send(SpawnActorEvent {
        transform: Transform::from_translation(spawn_point.get_spawn_point(&level)),
        ..default()
    });
}

pub fn spawn_world_anchor(
    mut commands: Commands,
    res: Res<WorldAnchorResources>,
    mut spawn_requests: EventReader<SpawnActorEvent<SpawnWorldAnchor>>,
    settings: Res<Settings>,
    _children_query: Query<&Children>,
) {
    for spawn in spawn_requests.read() {
        commands
            .spawn((
                StateScoped(LevelLoadState::Loaded),
                SceneRoot(res.scene.clone_weak()),
                spawn.transform.with_scale(Vec3::new(2.0, 2.0, 2.0)),
                Name::new("world anchor"),
                CombatantBundle {
                    combatant: Combatant::new(10., 0.),
                    death_info: DeathInfo {
                        death_type: super::DeathType::Immortal,
                    },
                    ..default()
                },
                PhysicsBundle {
                    //center of anchor is at bottom of model, so spawn the collision box offset
                    collider: Aabb::new(Vec3::new(2.0, 2.0, 2.0), Vec3::new(-1.0, 0.0, -1.0)),
                    mass: Mass(f32::INFINITY),
                    ..default()
                },
                WorldAnchor,
                ActiveWorldAnchor,
                ChunkLoader {
                    mesh: false,
                    ..settings.init_loader.clone()
                }, //no UninitializedActor b/c we don't have to do any setup
            ))
            .observe(observe_death);
    }
}

fn active_on_day(
    calendar: Res<Calendar>,
    query: Query<Entity, (With<WorldAnchor>, Without<ActiveWorldAnchor>)>,
    mut commands: Commands,
    mut prev_is_day: Local<bool>,
) {
    let is_day = calendar.in_day();
    if is_day && !*prev_is_day {
        for entity in query.iter() {
            commands.entity(entity).insert(ActiveWorldAnchor);
        }
    }
    *prev_is_day = is_day;
}

fn observe_death(trigger: Trigger<DeathEvent>, mut commands: Commands) {
    if let Some(mut ec) = commands.get_entity(trigger.entity()) {
        ec.remove::<ActiveWorldAnchor>();
    }
}

fn on_world_anchor_destroyed(
    _trigger: Trigger<OnRemove, WorldAnchor>,
    active_query: Query<&GlobalTransform, With<ActiveWorldAnchor>>,
    inactive_query: Query<&GlobalTransform, (With<WorldAnchor>, Without<ActiveWorldAnchor>)>,
    mut spawn: ResMut<SpawnPoint>,
) {
    info!("world anchor destoryed (probably picked up!!!!!");
    if !active_query.is_empty() {
        //set spawn point to some other query
        for gtf in active_query.iter() {
            info!(
                "world anchor destroyed, spawn point updated to active anchor at {:?}",
                gtf.translation()
            );
            spawn.base_point = gtf.translation();
        }
    } else if !inactive_query.is_empty() {
        //set spawn point to some other query
        for gtf in inactive_query.iter() {
            info!(
                "world anchor destroyed, spawn point updated to inactive anchor at {:?}",
                gtf.translation()
            );
            spawn.base_point = gtf.translation();
        }
    } else {
        // reset
        *spawn = SpawnPoint::default();
        info!(
            "world anchor destroyed, spawn point updated to default at {:?}",
            spawn.base_point
        );
    }
}

fn set_spawn_on_add(
    query: Query<&GlobalTransform, Added<ActiveWorldAnchor>>,
    mut spawn: ResMut<SpawnPoint>,
) {
    for gtf in query.iter() {
        info!(
            "world anchor spawned, spawn point updated to {:?}",
            gtf.translation()
        );
        spawn.base_point = gtf.translation();
    }
}
