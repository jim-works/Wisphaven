use core::f32;
use std::f32::consts::PI;

use super::spawning::*;
use bevy::prelude::*;
use engine::{
    actors::{
        team::EnemyTeam, AggroPlayer, AggroTargets, Combatant, CombatantBundle, ContactDamage,
        Damage, MoveSpeed,
    },
    controllers::{ControllableBundle, TickMovement},
    items::{
        loot::{ItemLootTable, ItemLootTableDrop},
        ItemName,
    },
};

use interfaces::scheduling::{LevelLoadState, LevelSystemSet, PhysicsLevelSet};
use physics::{
    collision::{Aabb, CollidingDirections, IgnoreTerrainCollision, TerrainQueryPoint},
    movement::{Drag, GravityMult, LookInMovementDirection, Velocity},
    PhysicsBundle,
};
use world::{chunk::ChunkCoord, chunk_loading::ChunkLoader};

pub struct SlitherSpinePlugin;

impl Plugin for SlitherSpinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(
                FixedUpdate,
                (spawn_handler, update_segments)
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
    default: DefaultSpawnArgs,
    segment_count: usize,
    segment_offset: Vec3,
}

impl Default for SpawnSlitherSpineEvent {
    fn default() -> Self {
        Self {
            default: DefaultSpawnArgs {
                transform: Transform::from_translation(Vec3::new(0., 10., -10.)),
            },
            segment_count: 15,
            segment_offset: Vec3::Z,
        }
    }
}

impl From<DefaultSpawnArgs> for SpawnSlitherSpineEvent {
    fn from(value: DefaultSpawnArgs) -> Self {
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
                    *attach_tf,
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
        let mut prev: Option<(Entity, Entity)> = None;
        for i in 0..spawn_event.segment_count {
            let offset = i as f32 * spawn_event.segment_offset;
            prev = Some(if let Some((prev_segment, head)) = prev {
                (
                    spawn_segement(
                        &mut commands,
                        resources.spine_scene.clone(),
                        spawn_event
                            .default
                            .transform
                            .with_translation(spawn_event.default.transform.translation + offset),
                        SlitherSpineSegment {
                            target_dist: segment_length,
                            parent: prev_segment,
                        },
                        head,
                    ),
                    head,
                )
            } else {
                let head = spawn_head(
                    &mut commands,
                    resources.head_scene.clone(),
                    spawn_event
                        .default
                        .transform
                        .with_translation(spawn_event.default.transform.translation + offset),
                );
                (head, head)
            });
        }
    }
}

fn spawn_head(commands: &mut Commands, scene: Handle<Scene>, transform: Transform) -> Entity {
    commands
        .spawn((
            StateScoped(LevelLoadState::Loaded),
            SceneRoot(scene),
            transform,
            PhysicsBundle {
                gravity: GravityMult::new(1.0),
                collider: Aabb::new(
                    Vec3::new(1.8, 1.0, 1.3) * transform.scale,
                    Vec3::new(-0.9, 0.2, -0.7) * transform.scale,
                ),
                drag: Drag(0.01),
                ..default()
            },
            Name::new("slither_spine_head"),
            SlitherSpineHead {
                in_ground_gravity_mult: -0.7,
                in_air_gravity_mult: 1.0,
                exit_ground_speed: 2.0,
                was_in_ground: false,
            },
            LookInMovementDirection(Quat::from_euler(EulerRot::XYZ, PI, PI, PI)),
            ControllableBundle {
                move_speed: MoveSpeed::new(0.005, 0.005, 0.5),
                ..default()
            },
            IgnoreTerrainCollision,
            TerrainQueryPoint,
            AggroTargets::new(vec![]),
            AggroPlayer {
                range: f32::INFINITY,
                priority: 0,
            },
            ChunkLoader {
                radius: ChunkCoord::new(1, 1, 1),
                lod_levels: 0,
                mesh: false,
            },
            CombatantBundle::<EnemyTeam> {
                combatant: Combatant::new(10., 1.),
                ..default()
            },
            ContactDamage::new(Damage::new(5.0)),
        ))
        .id()
}

fn spawn_segement(
    commands: &mut Commands,
    scene: Handle<Scene>,
    transform: Transform,
    segment: SlitherSpineSegment,
    head: Entity,
) -> Entity {
    commands
        .spawn((
            StateScoped(LevelLoadState::Loaded),
            SceneRoot(scene),
            transform,
            PhysicsBundle {
                gravity: GravityMult::new(0.0),
                collider: Aabb::new(
                    Vec3::new(1.8, 1.0, 1.0) * transform.scale,
                    Vec3::new(-0.9, 0.1, -0.5) * transform.scale,
                ),
                ..default()
            },
            Name::new("slither_spine_segment"),
            segment,
            IgnoreTerrainCollision,
            CombatantBundle::<EnemyTeam> {
                combatant: Combatant::new_child(head, 0.),
                ..default()
            },
            ContactDamage::new(Damage::new(1.0)),
            ItemLootTable {
                drops: vec![ItemLootTableDrop {
                    item: ItemName::core("tnt"),
                    drop_chance: 1.0,
                    drop_count_range: (1, 5),
                }],
            },
        ))
        .id()
}
fn move_head(
    mut head_query: Query<(
        &mut GravityMult,
        &mut Velocity,
        &mut TickMovement,
        &mut SlitherSpineHead,
        &CollidingDirections,
        &Transform,
        &AggroTargets,
    )>,
    aggro_query: Query<&GlobalTransform>,
) {
    for (mut g, mut v, mut movement, mut head, colliding_dirs, tf, targets) in head_query.iter_mut()
    {
        let in_ground = !colliding_dirs.0.is_empty();
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
        if let Some(aggro_gtf) = targets
            .current_target()
            .and_then(|e| aggro_query.get(e).ok())
        {
            let mut delta = aggro_gtf.translation() - tf.translation;
            delta.y = 0.;
            movement.0 = delta.normalize_or_zero();
        }
    }
}
