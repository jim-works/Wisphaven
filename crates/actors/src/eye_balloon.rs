use bevy::prelude::*;
use big_brain::{prelude::Highest, scorers::FixedScore, thinker::Thinker};
use engine::{
    actors::{
        ActorName, AggroPlayer, AggroTargets, BuildActorRegistry, Combatant, CombatantBundle,
        Defense, Health, MoveSpeed, SpawnActorEvent, ai::FlyToCurrentTargetAction,
        team::ENEMY_TEAM,
    },
    controllers::ControllableBundle,
};
use interfaces::scheduling::LevelSystemSet;
use physics::{
    PhysicsBundle,
    collision::{Aabb, IgnoreTerrainCollision},
    movement::{GravityMult, Mass},
};
use serde::Deserialize;
use util::{plugin::SmoothLookTo, third_party::scene_hook::SceneHook};

use crate::util::SmoothLookToAggroTarget;

#[derive(Resource)]
struct EyeBalloonResources {
    scene: Handle<Scene>,
}

pub struct EyeBalloonPlugin;

impl Plugin for EyeBalloonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources)
            .add_systems(
                FixedUpdate,
                spawn_eye_balloon.in_set(LevelSystemSet::PostTick),
            )
            .add_actor::<SpawnEyeBalloon>(ActorName::core("eye_balloon"));
    }
}

#[derive(Default, Debug, Deserialize)]
pub struct SpawnEyeBalloon {}

#[derive(Component)]
struct EyeBalloon;

#[derive(Component)]
struct EyeBalloonTentacle;

#[derive(Component)]
struct EyeBalloonTentacleSegment;

#[derive(Component)]
struct EyeBalloonIris;

fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(EyeBalloonResources {
        scene: assets.load("actors/eye_balloon/eye_balloon.glb#Scene0"),
    });
}

fn spawn_eye_balloon(
    mut commands: Commands,
    res: Res<EyeBalloonResources>,
    mut spawn_requests: EventReader<SpawnActorEvent<SpawnEyeBalloon>>,
) {
    for req in spawn_requests.read() {
        let mut head_ec = commands.spawn_empty();
        let head_id = head_ec.id();

        head_ec.insert((
            SceneRoot(res.scene.clone_weak()),
            req.transform,
            Name::new("eye_balloon"),
            PhysicsBundle {
                collider: Aabb::centered(Vec3::splat(1.5)),
                gravity: GravityMult(0.),
                mass: Mass(0.5),
                ..default()
            },
            ControllableBundle {
                mode: engine::controllers::MovementMode::Flying,
                move_speed: MoveSpeed::new(0.1, 0.1, 0.05),
                ..default()
            },
            AggroPlayer::default(),
            CombatantBundle {
                combatant: Combatant::Root {
                    health: Health::new(10.),
                    defense: Defense::new(0.),
                },
                team: ENEMY_TEAM,
                ..default()
            },
            AggroTargets::default(),
            EyeBalloon,
            Thinker::build()
                .label("eye_balloon_thinker")
                .picker(Highest)
                .when(FixedScore::build(0.01), FlyToCurrentTargetAction::default()),
            SceneHook::new(move |entity, ec| {
                match entity.get::<Name>().map(|t| t.as_str()) {
                    Some("Iris") => {
                        ec.insert((
                            EyeBalloonIris,
                            SmoothLookTo {
                                speed: 0.25,
                                ..default()
                            },
                            SmoothLookToAggroTarget { source: head_id },
                        ));
                    }
                    Some("Tentacle") => {
                        ec.insert((
                            Name::new("eye_balloon_tentacle"),
                            Aabb::new(
                                Vec3::new(0.75, 5., 0.75),
                                Vec3::new(-0.75 / 2., -5., -0.75 / 2.),
                            ),
                            IgnoreTerrainCollision,
                            Combatant::Child {
                                parent: head_id,
                                defense: Defense::default(),
                            },
                        ));
                    }
                    Some("TentacleEnd") => {
                        ec.insert((
                            EyeBalloonTentacle,
                            Aabb::centered(Vec3::splat(0.5)),
                            IgnoreTerrainCollision,
                            Combatant::Child {
                                parent: head_id,
                                defense: Defense::default(),
                            },
                        ));
                    }
                    Some("TentacleEndBone") => {}
                    _ => (),
                };
            }),
        ));
    }
}
