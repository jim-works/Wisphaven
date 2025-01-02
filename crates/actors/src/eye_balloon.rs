use bevy::prelude::*;
use big_brain::{prelude::Highest, scorers::FixedScore, thinker::Thinker};
use engine::{
    actors::{
        ai::FlyToCurrentTargetAction, team::EnemyTeam, AggroPlayer, AggroTargets, Combatant,
        CombatantBundle, Defense, Health, MoveSpeed,
    },
    controllers::ControllableBundle,
    physics::{
        collision::{Aabb, IgnoreTerrainCollision},
        movement::{GravityMult, Mass},
        PhysicsBundle,
    },
    world::{LevelLoadState, LevelSystemSet},
};
use util::{plugin::SmoothLookTo, third_party::scene_hook::SceneHook};

use crate::{
    spawning::{BuildActorRegistry, DefaultSpawnArgs, SpawnActorEvent},
    util::SmoothLookToAggroTarget,
};

#[derive(Resource)]
struct EyeBalloonResources {
    scene: Handle<Scene>,
}

pub struct EyeBalloonPlugin;

impl Plugin for EyeBalloonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resources).add_systems(
            FixedUpdate,
            spawn_eye_balloon.in_set(LevelSystemSet::PostTick),
        );
        // .add_systems(OnEnter(LevelLoadState::Loaded), test_spawn);

        app.add_event::<SpawnEyeBalloonEvent>();
        app.add_actor::<SpawnEyeBalloonEvent>("eye_balloon".to_string());
    }
}

#[derive(Event)]
pub struct SpawnEyeBalloonEvent {
    pub default_args: DefaultSpawnArgs,
}

impl From<DefaultSpawnArgs> for SpawnEyeBalloonEvent {
    fn from(value: DefaultSpawnArgs) -> Self {
        Self {
            default_args: value,
        }
    }
}

#[derive(Component)]
struct EyeBalloon;

#[derive(Component)]
struct EyeBalloonTentacle;

#[derive(Component)]
struct EyeBalloonTentacleSegment;

#[derive(Component)]
struct EyeBalloonIris;

fn test_spawn(mut writer: EventWriter<SpawnActorEvent>) {
    writer.send(SpawnActorEvent {
        name: std::sync::Arc::new("eye_balloon".to_string()),
        args: DefaultSpawnArgs {
            transform: Transform::from_translation(Vec3::new(0., 20., 0.)),
        },
    });
}

fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(EyeBalloonResources {
        scene: assets.load("actors/eye_balloon/eye_balloon.glb#Scene0"),
    });
}

fn spawn_eye_balloon(
    mut commands: Commands,
    res: Res<EyeBalloonResources>,
    mut spawn_requests: EventReader<SpawnEyeBalloonEvent>,
) {
    for SpawnEyeBalloonEvent { default_args } in spawn_requests.read() {
        let mut head_ec = commands.spawn_empty();
        let head_id = head_ec.id();

        head_ec.insert((
            SceneRoot(res.scene.clone_weak()),
            default_args.transform,
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
            CombatantBundle::<EnemyTeam> {
                combatant: Combatant::Root {
                    health: Health::new(10.),
                    defense: Defense::new(0.),
                },
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
