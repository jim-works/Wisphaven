use super::spawning::*;
use bevy::prelude::*;
use engine::world::LevelSystemSet;

pub struct SlitherSpinePlugin;

impl Plugin for SlitherSpinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(
                FixedUpdate,
                (trigger_spawn, spawn_handler)
                    .chain()
                    .in_set(LevelSystemSet::PostTick),
            )
            .add_event::<SpawnSlitherSpineEvent>();
    }
}

#[derive(Event)]
pub struct SpawnSlitherSpineEvent {
    default: DefaultSpawnEvent,
}

#[derive(Resource)]
struct SlitherSpineResources {
    spine_scene: Handle<Scene>,
}

#[derive(Component)]
struct SlitherSpine {}

fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SlitherSpineResources {
        spine_scene: assets.load("actors/slither_spine/spine_segment.glb#Scene0"),
    });
}

fn trigger_spawn(
    query: Query<(), With<SlitherSpine>>,
    mut send_events: EventWriter<SpawnSlitherSpineEvent>,
) {
    if query.is_empty() {
        send_events.send(SpawnSlitherSpineEvent {
            default: DefaultSpawnEvent {
                transform: Transform::from_translation(Vec3::new(0., 15., 0.)),
            },
        });
    }
}

fn spawn_handler(
    mut commands: Commands,
    resources: Res<SlitherSpineResources>,
    mut events: EventReader<SpawnSlitherSpineEvent>,
) {
    for spawn_event in events.read() {
        commands.spawn((
            SceneBundle {
                scene: resources.spine_scene.clone(),
                transform: spawn_event.default.transform,
                ..default()
            },
            Name::new("slither_spine"),
            SlitherSpine {},
        ));
    }
}
