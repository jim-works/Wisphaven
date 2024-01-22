use bevy::prelude::*;
use big_brain::prelude::*;

use crate::{
    physics::{collision::Aabb, movement::GravityMult, PhysicsBundle},
    ui::healthbar::spawn_billboard_healthbar,
    util::{plugin::SmoothLookTo, SendEventCommand},
    world::LevelLoadState,
};

use super::{
    behaviors::{FloatAction, FloatHeight, FloatScorer, FloatWander, FloatWanderAction},
    personality::components::*,
    ActorName, ActorResources, CombatInfo, CombatantBundle, DefaultAnimation, Idler,
    UninitializedActor,
};

#[derive(Resource)]
pub struct GlowjellyResources {
    pub scene: Handle<Scene>,
    pub anim: Handle<AnimationClip>,
}

#[derive(Component, Default)]
pub struct Glowjelly {
    scene: Option<Entity>,
    color: Color,
}

#[derive(Component)]
pub struct GlowjellyScene;

#[derive(Event)]
pub struct SpawnGlowjellyEvent {
    pub location: GlobalTransform,
    pub color: Color,
}

pub struct GlowjellyPlugin;

impl Plugin for GlowjellyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_systems(Update, (spawn_glowjelly, setup_glowjelly, social_score))
            .add_event::<SpawnGlowjellyEvent>();
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(GlowjellyResources {
        scene: assets.load("glowjelly/glowjelly.gltf#Scene0"),
        anim: assets.load("glowjelly/glowjelly.gltf#Animation0"),
    });
}

fn trigger_spawning(mut writer: EventWriter<SpawnGlowjellyEvent>) {
    for i in 0..5 {
        writer.send(SpawnGlowjellyEvent {
            location: GlobalTransform::from_xyz(i as f32 * 5.0, -45.0, 0.0),
            color: Color::rgb(i as f32, 1.0, 1.0),
        });
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("glowjelly"),
        Box::new(|commands, tf| {
            commands.add(SendEventCommand(SpawnGlowjellyEvent {
                location: tf,
                color: Color::RED,
            }))
        }),
    );
}

pub fn spawn_glowjelly(
    mut commands: Commands,
    jelly_res: Res<GlowjellyResources>,
    mut spawn_requests: EventReader<SpawnGlowjellyEvent>,
    _children_query: Query<&Children>,
) {
    for spawn in spawn_requests.read() {
        let id = commands
            .spawn((
                SceneBundle {
                    scene: jelly_res.scene.clone_weak(),
                    transform: spawn.location.compute_transform(),
                    ..default()
                },
                Name::new("glowjelly"),
                CombatantBundle {
                    combat_info: CombatInfo {
                        knockback_multiplier: 10.0,
                        ..CombatInfo::new(10.0, 0.0)
                    },
                    ..default()
                },
                PhysicsBundle {
                    collider: Aabb::new(Vec3::ONE, -0.5 * Vec3::ONE),
                    gravity: GravityMult(0.1),
                    ..default()
                },
                PersonalityBundle {
                    personality: PersonalityValues {
                        status: FacetValue::new(100.0, 1.0).unwrap(),
                        ..default()
                    },
                    ..default()
                },
                Glowjelly {
                    color: spawn.color,
                    ..default()
                },
                UninitializedActor,
                FloatHeight::new(20.0),
                Idler::default(),
                FloatWander::default(),
                SmoothLookTo::new(0.5),
                Thinker::build()
                    .label("glowjelly thinker")
                    .picker(FirstToScore::new(0.005))
                    .when(
                        FloatScorer,
                        FloatAction {
                            impulse: 5.0,
                            turn_speed: 0.5,
                        },
                    )
                    .when(
                        FixedScore::build(0.05),
                        FloatWanderAction {
                            impulse: 0.1,
                            squish_factor: Vec3::new(1.0, 0.33, 1.0),
                            anim_speed: 0.66,
                        },
                    ),
            ))
            .id();
        //add healthbar
        spawn_billboard_healthbar(&mut commands, id, Vec3::new(0.0, 2.0, 0.0));
    }
}

//TODO: extract into separate function. all scenes should have this setup
pub fn setup_glowjelly(
    mut commands: Commands,
    children_query: Query<&Children>,
    jelly_res: Res<GlowjellyResources>,
    mut glowjelly_query: Query<(Entity, &mut Glowjelly), With<UninitializedActor>>,
    anim_query: Query<&AnimationPlayer>,
    animations: Res<Assets<AnimationClip>>,
) {
    for (parent_id, mut glowjelly) in glowjelly_query.iter_mut() {
        //hierarchy is parent -> scene -> gltfnode (with animation player)
        //find first child with a child that has an animation player
        //we have to wait until the children get spawned in (aka the gltf loaded)
        if let Ok(children) = children_query.get(parent_id) {
            for child in children {
                let mut found = false;
                if let Ok(grandchildren) = children_query.get(*child) {
                    for candidate_anim_player in grandchildren {
                        if anim_query.contains(*candidate_anim_player) {
                            commands
                                .entity(*candidate_anim_player)
                                .insert(GlowjellyScene);
                            commands
                                .entity(parent_id)
                                .remove::<UninitializedActor>()
                                .insert(DefaultAnimation::new(
                                    jelly_res.anim.clone(),
                                    *candidate_anim_player,
                                    1.1,
                                    if let Some(clip) = animations.get(&jelly_res.anim) {
                                        clip.duration()
                                    } else {
                                        0.0
                                    },
                                ))
                                .with_children(|cb| {
                                    cb.spawn(PointLightBundle {
                                        point_light: PointLight {
                                            color: glowjelly.color,
                                            intensity: 100.0,
                                            shadows_enabled: true,
                                            ..default()
                                        },
                                        ..default()
                                    });
                                });
                            glowjelly.scene = Some(*candidate_anim_player);

                            found = true;
                            break;
                        } else {
                            error!("glowjelly animation player not found");
                        }
                    }
                }
                if found {
                    break;
                }
            }
        }
    }
}

#[derive(Clone, Component, Debug, ScorerBuilder)]
pub struct SocialScorer;

fn social_score(
    mut query: Query<(&mut FloatHeight, &GlobalTransform)>,
    friend_query: Query<&GlobalTransform, With<Glowjelly>>,
) {
    const SQUARE_RADIUS: f32 = 25.0*25.0;
    for (mut height, tf) in query.iter_mut() {
        let mut sum_height_diff = 0.0;
        let mut count = 0.0;
        //todo: optimize
        for friend_tf in friend_query.iter() {
            if friend_tf.translation().distance_squared(tf.translation()) < SQUARE_RADIUS {
                sum_height_diff += tf.translation().y - friend_tf.translation().y;
                count += 1.0;
            }
        }
        height.task.outcomes.status = if count == 0.0 {
            0.0
        } else {
            -sum_height_diff / count
        }
    }
}
