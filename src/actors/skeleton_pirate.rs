use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use big_brain::prelude::*;

use crate::{
    actors::{
        ai::{scorers::AggroLineOfSightScorer, AttackAction},
        AggroTargets,
    },
    controllers::ControllableBundle,
    physics::PhysicsObjectBundle,
    util::SendEventCommand,
    world::LevelLoadState,
};

use super::{
    ai::WalkToDestinationAction, coin::SpawnCoinEvent, world_anchor::WorldAnchor, ActorName,
    ActorResources, CombatInfo, CombatantBundle, Damage, DefaultAnimation, Jump, MoveSpeed,
    UninitializedActor,
};

#[derive(Resource)]
pub struct SkeletonPirateResources {
    pub scene: Handle<Scene>,
    pub anim: Handle<AnimationClip>,
}

#[derive(Component, Default)]
pub struct SkeletonPirate {
    scene: Option<Entity>,
}

#[derive(Component)]
pub struct SkeletonPirateScene;

#[derive(Event)]
pub struct SpawnSkeletonPirateEvent {
    pub location: GlobalTransform,
}

pub struct SkeletonPiratePlugin;

impl Plugin for SkeletonPiratePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_systems(PreUpdate, attack.in_set(BigBrainSet::Actions))
            .add_systems(Update, spawn_skeleton_pirate)
            .add_event::<SpawnSkeletonPirateEvent>();
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SkeletonPirateResources {
        scene: assets.load("skeletons/skeleton_pirate.gltf#Scene0"),
        anim: assets.load("skeletons/skeleton_pirate.gltf#Animation0"),
    });
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("skeleton_pirate"),
        Box::new(|commands, tf| {
            commands.add(SendEventCommand(SpawnSkeletonPirateEvent { location: tf }))
        }),
    );
}

fn trigger_spawning(mut writer: EventWriter<SpawnSkeletonPirateEvent>) {
    writer.send(SpawnSkeletonPirateEvent {
        location: GlobalTransform::from_xyz(10.0, 25.0, 0.0),
    });
}

pub fn spawn_skeleton_pirate(
    mut commands: Commands,
    skele_res: Res<SkeletonPirateResources>,
    mut spawn_requests: EventReader<SpawnSkeletonPirateEvent>,
    anchor: Query<(Entity, &GlobalTransform), With<WorldAnchor>>,
) {
    let (anchor_entity, anchor_position) = anchor
        .get_single()
        .ok()
        .map(|(e, tf)| (e, tf.translation()))
        .unwrap_or((Entity::PLACEHOLDER, Vec3::ZERO));
    const ATTACK_RANGE: f32 = 20.0;
    for spawn in spawn_requests.iter() {
        commands.spawn((
            SceneBundle {
                scene: skele_res.scene.clone_weak(),
                transform: spawn.location.compute_transform(),
                ..default()
            },
            Name::new("SkeletonPirate"),
            CombatantBundle {
                combat_info: CombatInfo::new(10.0, 0.0),
                ..default()
            },
            PhysicsObjectBundle {
                rigidbody: RigidBody::Dynamic,
                collider: Collider::capsule(Vec3::new(0., 0.5, 0.), Vec3::new(0., 2.4, 0.), 0.5),
                ..default()
            },
            Friction {
                coefficient: 0.2,
                combine_rule: CoefficientCombineRule::Min,
            },
            ControllableBundle {
                jump: Jump::new(12.0, 0),
                move_speed: MoveSpeed::new(50.0, 10.0, 5.0),
                ..default()
            },
            SkeletonPirate { ..default() },
            AggroTargets(vec![anchor_entity]),
            UninitializedActor,
            Thinker::build()
                .label("skeleton thinker")
                .picker(Highest)
                .when(
                    FixedScore::build(0.01),
                    WalkToDestinationAction {
                        target_dest: anchor_position,
                        stop_distance: ATTACK_RANGE * 0.5,
                        ..default()
                    },
                )
                .when(
                    AggroLineOfSightScorer {
                        range: ATTACK_RANGE,
                    },
                    AttackAction,
                ),
        ));
    }
}

//TODO: extract into separate function. all scenes should have this setup
pub fn setup_skeleton_pirate(
    mut commands: Commands,
    children_query: Query<&Children>,
    skele_res: Res<SkeletonPirateResources>,
    mut skeleton_pirate_query: Query<(Entity, &mut SkeletonPirate), With<UninitializedActor>>,
    anim_query: Query<&AnimationPlayer>,
    animations: Res<Assets<AnimationClip>>,
) {
    for (parent_id, mut skele) in skeleton_pirate_query.iter_mut() {
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
                                .insert(SkeletonPirateScene);
                            commands
                                .entity(parent_id)
                                .remove::<UninitializedActor>()
                                .insert(DefaultAnimation::new(
                                    skele_res.anim.clone(),
                                    *candidate_anim_player,
                                    1.1,
                                    if let Some(clip) = animations.get(&skele_res.anim) {
                                        clip.duration()
                                    } else {
                                        0.0
                                    },
                                ));
                            skele.scene = Some(*candidate_anim_player);

                            found = true;
                            break;
                        } else {
                            error!("SkeletonPirate animation player not found");
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

fn attack(
    mut action_query: Query<(&Actor, &mut ActionState), With<AttackAction>>,
    skele_query: Query<(&GlobalTransform, &AggroTargets), With<SkeletonPirate>>,
    aggro_query: Query<&GlobalTransform>,
    mut spawn_coin: EventWriter<SpawnCoinEvent>,
) {
    const COIN_OFFSET: Vec3 = Vec3::new(0.0, 2.0, 0.0);
    const THROW_IMPULSE: f32 = 5.0;
    let combat = CombatantBundle::default();
    let damage = Damage::default();
    for (&Actor(actor), mut state) in action_query.iter_mut() {
        match *state {
            ActionState::Requested => {
                info!("throwing!");
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                if let Ok((tf, targets)) = skele_query.get(actor) {
                    if let Some(target) = targets.last().map(|t| aggro_query.get(*t).ok()).flatten()
                    {
                        let spawn_point = tf.translation() + COIN_OFFSET;
                        spawn_coin.send(SpawnCoinEvent {
                            location: Transform::from_translation(spawn_point),
                            velocity: (target.translation() - spawn_point).normalize_or_zero()
                                * THROW_IMPULSE,
                            combat: combat.clone(),
                            owner: actor,
                            damage,
                        })
                    }
                }
                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            },
            _ => {}
        }
    }
}
