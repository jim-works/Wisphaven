use bevy::prelude::*;

use engine::{
    physics::{collision::Aabb, movement::GravityMult, PhysicsBundle},
    world::LevelLoadState,
};

use util::{plugin::SmoothLookTo, SendEventCommand};

use engine::actors::{
    team::PlayerTeam, ActorName, ActorResources, Combatant, CombatantBundle, Idler,
};

#[derive(Resource)]
pub struct WispResources {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

#[derive(Component, Default)]
pub struct Wisp;

#[derive(Event)]
pub struct SpawnWispEvent {
    pub location: Transform,
}

pub struct WispPlugin;

impl Plugin for WispPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_systems(Update, spawn_wisp)
            .add_event::<SpawnWispEvent>();
    }
}

pub fn load_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(WispResources {
        mesh: meshes.add(Cuboid::from_length(1.0)),
        material: materials.add(StandardMaterial::from(Color::WHITE)),
    });
}

fn trigger_spawning(mut writer: EventWriter<SpawnWispEvent>) {
    for i in 0..0 {
        writer.send(SpawnWispEvent {
            location: Transform::from_xyz(
                (i % 5) as f32 * -5.0,
                (i / 5) as f32 * 5.0 + 50.0,
                (i / 5) as f32 * -1.0 + 10.0,
            ),
        });
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("wisp"),
        Box::new(|commands, tf| commands.queue(SendEventCommand(SpawnWispEvent { location: tf }))),
    );
}

fn spawn_wisp(
    mut commands: Commands,
    res: Res<WispResources>,
    mut spawn_requests: EventReader<SpawnWispEvent>,
) {
    for spawn in spawn_requests.read() {
        commands.spawn((
            Mesh3d(res.mesh.clone()),
            MeshMaterial3d(res.material.clone()),
            spawn.location,
            Name::new("wisp"),
            CombatantBundle::<PlayerTeam> {
                combatant: Combatant::new(10., 0.),
                ..default()
            },
            PhysicsBundle {
                collider: Aabb::centered(Vec3::splat(1.0)),
                gravity: GravityMult::new(0.5),
                ..default()
            },
            Wisp,
            Idler::default(),
            SmoothLookTo::new(0.5),
            bevy::pbr::CubemapVisibleEntities::default(),
            bevy::render::primitives::CubemapFrusta::default(),
        ));
    }
}
