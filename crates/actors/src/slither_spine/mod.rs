use std::f32::consts::PI;

use super::spawning::*;
use bevy::prelude::*;
use engine::{
    actors::{LocalPlayer, MoveSpeed},
    controllers::{ControllableBundle, TickMovement},
    physics::{
        collision::{CollidingDirections, IgnoreTerrainCollision, TerrainQueryPoint},
        movement::{Acceleration, GravityMult, LookInMovementDirection, Velocity},
        PhysicsBundle, PhysicsLevelSet, GRAVITY,
    },
    world::{Level, LevelSystemSet},
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
            .add_systems(FixedUpdate, move_head.in_set(PhysicsLevelSet::Main))
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
struct SlitherSpineHead {
    in_ground_gravity_mult: f32,
    in_air_gravity_mult: f32,
    exit_ground_speed: f32,
    was_in_ground: bool,
}

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
            segment_offset: Vec3::new(0., 0., -1.),
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
                    attach_tf.clone(),
                ));
            }
        }
    }
    let mut tf_query = set.p1();
    for (entity, dx, target) in new_translations.into_iter() {
        if let Ok(mut tf) = tf_query.get_mut(entity) {
            tf.translation += dx;
            tf.look_at(target.translation, target.up());
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
                        IgnoreTerrainCollision,
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
                            gravity: GravityMult::new(1.0),
                            ..default()
                        },
                        Name::new("slither_spine_head"),
                        SlitherSpineHead {
                            in_ground_gravity_mult: -1.5,
                            in_air_gravity_mult: 1.0,
                            exit_ground_speed: 1.0,
                            was_in_ground: false,
                        },
                        LookInMovementDirection(Quat::from_euler(EulerRot::XYZ, PI, PI, PI)),
                        ControllableBundle {
                            move_speed: MoveSpeed::new(0.01, 0.01, 0.5),
                            ..default()
                        },
                        IgnoreTerrainCollision,
                        TerrainQueryPoint,
                    ))
                    .id()
            });
        }
    }
}

fn move_head(
    level: Res<Level>,
    mut head_query: Query<(
        &mut GravityMult,
        &mut Velocity,
        &mut TickMovement,
        &Transform,
        &mut SlitherSpineHead,
    )>,
    local_player_query: Query<&Transform, With<LocalPlayer>>,
) {
    let Ok(player_tf) = local_player_query.get_single() else {
        return;
    };
    for (mut g, mut v, mut movement, tf, mut head) in head_query.iter_mut() {
        let in_ground = level.get_block_entity(tf.translation.into()).is_some();
        if in_ground {
            g.0 = head.in_ground_gravity_mult;
        } else {
            g.0 = head.in_air_gravity_mult;
        }
        if head.was_in_ground && !in_ground {
            //exited ground, add burst of speed
            v.0.y = head.exit_ground_speed;
        }
        head.was_in_ground = in_ground;
        let mut delta = player_tf.translation - tf.translation;
        delta.y = 0.;
        movement.0 = delta.normalize_or_zero();
    }
}
