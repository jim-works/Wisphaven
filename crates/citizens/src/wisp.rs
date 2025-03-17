use std::f32::consts::PI;

use bevy::prelude::*;

use interfaces::{
    resources::HeldItemResources,
    scheduling::{LevelLoadState, LevelSystemSet},
};
use physics::{
    PhysicsBundle,
    collision::Aabb,
    movement::{GravityMult, Mass},
};
use util::{SendEventCommand, lerp, plugin::SmoothLookTo};

use engine::{
    actors::{
        ActorName, ActorResources, Combatant, CombatantBundle, Idler,
        ghost::{Float, GhostResources, Handed, OrbitParticle},
        team::PlayerTeam,
    },
    items::{ItemName, ItemResources, ItemStack, inventory::Inventory},
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
    pub handed: Handed,
}

pub struct WispPlugin;

impl Plugin for WispPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(FixedUpdate, spawn_wisp.in_set(LevelSystemSet::Tick))
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

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("wisp"),
        Box::new(|commands, tf| {
            commands.queue(SendEventCommand(SpawnWispEvent {
                location: tf,
                handed: Handed::Left,
            }))
        }),
    );
}

fn spawn_wisp(
    mut commands: Commands,
    res: Res<GhostResources>,
    items: Res<ItemResources>,
    held_item_resources: Res<HeldItemResources>,
    mut spawn_requests: EventReader<SpawnWispEvent>,
) {
    const MIN_PARTICLE_SIZE: f32 = 0.225;
    const MAX_PARTICLE_SIZE: f32 = 0.7;
    const MIN_PARTICLE_DIST: f32 = 0.15;
    const MAX_PARTICLE_DIST: f32 = 0.5;
    const MIN_PARTICLE_SPEED: f32 = 0.05;
    const MAX_PARTICLE_SPEED: f32 = 0.2;
    const PARTICLE_COUNT: u32 = 7;
    for spawn in spawn_requests.read() {
        let ghost_entity = commands
            .spawn((
                StateScoped(LevelLoadState::Loaded),
                MeshMaterial3d(res.material.clone()),
                Mesh3d(res.center_mesh.clone()),
                spawn
                    .location
                    .with_translation(spawn.location.translation + Vec3::Y),
                Name::new("ghost"),
                CombatantBundle::<PlayerTeam> {
                    combatant: Combatant::new(10.0, 0.),
                    ..default()
                },
                PhysicsBundle {
                    collider: Aabb::centered(Vec3::new(0.8, 1.0, 0.8)),
                    mass: Mass(0.5),
                    ..default()
                },
                Float::default(),
                Wisp,
                Idler::default(),
                SmoothLookTo::new(0.5),
            ))
            .with_children(|children| {
                //orbit particles
                for (i, point) in (0..PARTICLE_COUNT)
                    .zip(util::iterators::even_distribution_on_sphere(PARTICLE_COUNT))
                {
                    //size and distance are inversely correlated
                    let size = lerp(
                        MAX_PARTICLE_SIZE,
                        MIN_PARTICLE_SIZE,
                        i as f32 / PARTICLE_COUNT as f32,
                    );
                    let dist = lerp(
                        MIN_PARTICLE_DIST,
                        MAX_PARTICLE_DIST,
                        i as f32 / PARTICLE_COUNT as f32,
                    );
                    let speed = lerp(
                        MIN_PARTICLE_SPEED,
                        MAX_PARTICLE_SPEED,
                        i as f32 / PARTICLE_COUNT as f32,
                    );
                    let material = res.particle_materials[i as usize].clone();
                    let angle_inc = 2.0 * PI / PARTICLE_COUNT as f32;
                    let angle = i as f32 * angle_inc;
                    children.spawn((
                        MeshMaterial3d(material),
                        Mesh3d(res.particle_mesh.clone()),
                        Transform::from_translation(point * dist).with_scale(Vec3::splat(size)),
                        OrbitParticle::stable(
                            dist,
                            Vec3::new(speed * angle.sin(), 0.0, speed * angle.cos()),
                        ),
                    ));
                }
            })
            .id();
        let mut inventory = Inventory::new(ghost_entity, 5);
        inventory.set_slot_no_events(
            0,
            ItemStack::new(
                items
                    .registry
                    .get_basic(&ItemName::core("ruby_pickaxe"))
                    .unwrap(),
                1,
            ),
        );
        commands.entity(ghost_entity).insert(inventory);
        //right hand
        let right_hand_entity = engine::actors::ghost::spawn_ghost_hand(
            ghost_entity,
            spawn.location,
            Vec3::new(0.5, -0.2, -0.6),
            Vec3::new(0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &res,
            &mut commands,
        );
        //left hand
        let left_hand_entity = engine::actors::ghost::spawn_ghost_hand(
            ghost_entity,
            spawn.location,
            Vec3::new(-0.5, -0.2, -0.6),
            Vec3::new(-0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &res,
            &mut commands,
        );
        spawn.handed.assign_hands(
            ghost_entity,
            left_hand_entity,
            right_hand_entity,
            &mut commands,
        );
        let item_visualizer = held_item_resources.create_held_item_visualizer(
            &mut commands,
            ghost_entity,
            Transform::from_scale(Vec3::splat(4.0)).with_translation(Vec3::new(0.0, -1.0, -3.4)),
        );
        commands
            .entity(right_hand_entity)
            .add_child(item_visualizer);
    }
}
