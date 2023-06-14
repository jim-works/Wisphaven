use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use big_brain::prelude::*;

use crate::physics::PhysicsObjectBundle;

use super::{CombatInfo, CombatantBundle};

#[derive(Resource)]
pub struct GlowjellyResources {
    pub anim: Handle<AnimationClip>,
    pub scene: Handle<Scene>,
}

#[derive(Component)]
pub struct Glowjelly;

pub struct SpawnGlowjellyEvent {
    pub location: Transform,
    pub color: Color,
}
#[derive(Component, Debug)]
pub struct FloatHeight {
    pub curr_height: f32,
    pub preferred_height: f32,
    pub seconds_remaining: f32,
}

impl FloatHeight {
    pub fn new(preferred_height: f32) -> Self {
        Self {
            curr_height: 0.0,
            preferred_height,
            seconds_remaining: 0.0
        }
    }
}

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct FloatAction;

#[derive(Clone, Component, Debug, ScorerBuilder)]
pub struct FloatScorer;

pub struct GlowjellyPlugin;

impl Plugin for GlowjellyPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(load_resources)
            .add_system(spawn_glowjelly)
            .add_system(keyboard_animation_control)
            .add_system(eval_height)
            .add_system(float_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(float_action_system.in_set(BigBrainSet::Actions))
            .add_event::<SpawnGlowjellyEvent>();
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(GlowjellyResources {
        anim: assets.load("glowjelly/glowjelly.gltf#Animation0"),
        scene: assets.load("glowjelly/glowjelly.gltf#Scene0"),
    });
}

pub fn spawn_glowjelly(
    mut commands: Commands,
    jelly_res: Res<GlowjellyResources>,
    mut spawn_requests: EventReader<SpawnGlowjellyEvent>,
) {
    for spawn in spawn_requests.iter() {
        commands
            .spawn((
                SceneBundle {
                    scene: jelly_res.scene.clone_weak(),
                    transform: spawn.location,
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
                PhysicsObjectBundle {
                    rigidbody: RigidBody::Dynamic,
                    collider: Collider::cuboid(0.5, 0.5, 0.5),
                    ..default()
                },
                GravityScale(0.1),
                Glowjelly,
                FloatHeight::new(20.0),
                Thinker::build()
                    .label("glowjelly thinker")
                    .picker(FirstToScore {threshold: 0.5})
                    .when(FloatScorer, FloatAction)
            ))
            .with_children(|cb| {
                cb.spawn(PointLightBundle {
                    point_light: PointLight {
                        color: spawn.color,
                        intensity: 500.0,
                        shadows_enabled: true,
                        ..default()
                    },
                    ..default()
                });
            });
    }
}
pub fn eval_height (
    collision: Res<RapierContext>,
    mut query: Query<(&mut FloatHeight, &GlobalTransform)>
) {
    let groups = QueryFilter {
        groups: Some(CollisionGroups::new(
        Group::ALL,
            Group::from_bits_truncate(crate::physics::TERRAIN_GROUP),
        )),
        ..default()
    };
    for (mut height, tf) in query.iter_mut() {
        height.curr_height = if let Some((_,dist)) = collision.cast_ray(tf.translation(), Vec3::NEG_Y, height.preferred_height, true, groups) {
            dist
        } else {
            height.preferred_height
        };
    }
}
pub fn float_action_system (
    time: Res<Time>,
    jelly_anim: Res<GlowjellyResources>,
    mut info: Query<(&mut FloatHeight, &mut ExternalImpulse)>,
    mut query: Query<(&Actor, &mut ActionState), With<FloatAction>>
) {
    for (Actor(actor), mut state) in query.iter_mut() {
        if let Ok((mut floater, mut impulse)) = info.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    *state = ActionState::Executing;
                    impulse.impulse += Vec3::Y*5.0;
                    floater.seconds_remaining = 5.0;
                }
                ActionState::Executing => {
                    floater.seconds_remaining -= time.delta_seconds();
                    if floater.seconds_remaining <= 0.0 {
                        *state = ActionState::Success;
                    }
                }
                // All Actions should make sure to handle cancellations!
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

pub fn float_scorer_system (
    floats: Query<&FloatHeight>,
    mut query: Query<(&Actor, &mut Score), With<FloatScorer>>
) {
    for (Actor(actor), mut score) in query.iter_mut() {
        if let Ok(float) = floats.get(*actor) {
            score.set((float.preferred_height-float.curr_height)/float.preferred_height);
        }
    }
}

fn keyboard_animation_control(
    keyboard_input: Res<Input<KeyCode>>,
    jelly_anim: Res<GlowjellyResources>,
    mut animation_player: Query<&mut AnimationPlayer>,
    mut impulse_query: Query<&mut ExternalImpulse, With<Glowjelly>>,
) {
    if keyboard_input.just_pressed(KeyCode::Return) {
        for mut anim_player in animation_player.iter_mut() {
            anim_player.start(jelly_anim.anim.clone_weak());
        }
        for mut impulse in impulse_query.iter_mut() {
            impulse.impulse += Vec3::new(0.0, 5.0, 0.0);
        }
    }
    
}
