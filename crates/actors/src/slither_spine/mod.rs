use std::f32::consts::PI;

use super::spawning::*;
use bevy::prelude::*;
use engine::{
    physics::{
        movement::{GravityMult, LookInMovementDirection},
        PhysicsBundle,
    },
    world::LevelSystemSet,
};

pub struct SlitherSpinePlugin;

impl Plugin for SlitherSpinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(
                FixedUpdate,
                (trigger_spawn, spawn_handler, update_segments)
                    .chain()
                    .in_set(LevelSystemSet::PostTick),
            )
            .add_event::<SpawnSlitherSpineEvent>()
            .add_actor::<SpawnSlitherSpineEvent>("slither_spine".to_string());
    }
}

#[derive(Event)]
pub struct SpawnSlitherSpineEvent {
    default: DefaultSpawnEvent,
    segment_count: usize,
    segment_offset: Vec3,
}

impl Default for SpawnSlitherSpineEvent {
    fn default() -> Self {
        Self {
            default: Default::default(),
            segment_count: 5,
            segment_offset: Vec3::Z,
        }
    }
}

impl From<DefaultSpawnEvent> for SpawnSlitherSpineEvent {
    fn from(value: DefaultSpawnEvent) -> Self {
        Self {
            default: value,
            ..default()
        }
    }
}

#[derive(Resource)]
struct SlitherSpineResources {
    spine_scene: Handle<Scene>,
    head_scene: Handle<Scene>,
}

#[derive(Component)]
struct SlitherSpineSegment {
    target_dist: f32,
    parent: Entity,
}

#[derive(Component)]
struct SlitherSpineHead;

fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SlitherSpineResources {
        spine_scene: assets.load("actors/slither_spine/spine_segment.glb#Scene0"),
        head_scene: assets.load("actors/slither_spine/spine_head.glb#Scene0"),
    });
}

fn trigger_spawn(
    query: Query<(), With<SlitherSpineSegment>>,
    mut send_events: EventWriter<SpawnSlitherSpineEvent>,
) {
    if query.is_empty() {
        send_events.send(SpawnSlitherSpineEvent {
            default: DefaultSpawnEvent {
                transform: Transform::from_translation(Vec3::new(0., 15., 0.)),
            },
            segment_count: 15,
            segment_offset: Vec3::new(1., 1., 0.),
        });
    }
}

fn update_segments(
    mut set: ParamSet<(
        (
            Query<(Entity, &Transform, &SlitherSpineSegment)>,
            Query<&Transform>,
        ),
        Query<&mut Transform>,
    )>,
) {
    let mut new_translations = Vec::with_capacity(set.p0().0.iter().len());
    {
        let (segments, attachments) = set.p0();
        for (entity, tf, segment) in segments.iter() {
            if let Ok(attach_tf) = attachments.get(segment.parent) {
                let delta = attach_tf.translation - tf.translation;
                new_translations.push((
                    entity,
                    delta - delta.normalize_or_zero() * segment.target_dist,
                    attach_tf.translation,
                ));
            }
        }
    }
    let mut tf_query = set.p1();
    for (entity, dx, target) in new_translations.into_iter() {
        if let Ok(mut tf) = tf_query.get_mut(entity) {
            tf.translation += dx;
            tf.look_at(target, Vec3::Y);
        }
    }
}

fn spawn_handler(
    mut commands: Commands,
    resources: Res<SlitherSpineResources>,
    mut events: EventReader<SpawnSlitherSpineEvent>,
) {
    for spawn_event in events.read() {
        let segment_length = spawn_event.segment_offset.length();
        let mut prev: Option<Entity> = None;
        for i in 0..spawn_event.segment_count {
            let offset = i as f32 * spawn_event.segment_offset;
            prev = Some(if let Some(prev_segment) = prev {
                commands
                    .spawn((
                        SceneBundle {
                            scene: resources.spine_scene.clone(),
                            transform: spawn_event.default.transform.with_translation(
                                spawn_event.default.transform.translation + offset,
                            ),
                            ..default()
                        },
                        PhysicsBundle {
                            gravity: GravityMult::new(0.0),
                            ..default()
                        },
                        Name::new("slither_spine_segment"),
                        SlitherSpineSegment {
                            target_dist: segment_length,
                            parent: prev_segment,
                        },
                    ))
                    .id()
            } else {
                commands
                    .spawn((
                        SceneBundle {
                            scene: resources.head_scene.clone(),
                            transform: spawn_event.default.transform.with_translation(
                                spawn_event.default.transform.translation + offset,
                            ),
                            ..default()
                        },
                        PhysicsBundle {
                            gravity: GravityMult::new(0.05),
                            ..default()
                        },
                        Name::new("slither_spine_head"),
                        SlitherSpineHead,
                        LookInMovementDirection(Quat::from_euler(EulerRot::XYZ, PI, PI, PI)),
                    ))
                    .id()
            });
        }
    }
}
