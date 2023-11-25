use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    mesher::{ArrayTextureMaterial, ChunkMaterial},
    physics::PhysicsObjectBundle,
    world::{
        events::ChunkUpdatedEvent, BlockCoord, BlockId, BlockMesh, BlockPhysics, BlockType, Level,
        LevelSystemSet,
    },
};

const HALF_SIDE: f32 = 0.45;
const MAX_PLANAR_VELOCITY: f32 = 0.25;

pub struct BlockActorPlugin;

impl Plugin for BlockActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnFallingBlockEvent>()
            .add_event::<LandedFallingBlockEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (falling_block_spawner, falling_block_placer, on_block_landed)
                    .in_set(LevelSystemSet::Main),
            );
    }
}

#[derive(Event)]
pub struct SpawnFallingBlockEvent {
    pub position: Vec3,
    pub initial_velocity: Vec3,
    pub block: Entity,
    pub place_on_landing: bool,
}

#[derive(Event)]
pub struct LandedFallingBlockEvent {
    pub position: BlockCoord,
    pub faller: Entity,
    pub block: Entity,
    pub place_on_landing: bool,
}

#[derive(Component)]
pub struct FallingBlock(Entity, bool);

#[derive(Resource)]
struct FallingBlockResources {
    default_mesh: Handle<Mesh>,
    default_material: Handle<StandardMaterial>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });
    commands.insert_resource(FallingBlockResources {
        default_mesh: mesh,
        default_material: material,
    });
}

fn falling_block_spawner(
    mut reader: EventReader<SpawnFallingBlockEvent>,
    mut commands: Commands,
    mesh_query: Query<&BlockMesh>,
    materials: Res<ChunkMaterial>,
    resources: Res<FallingBlockResources>,
) {
    for event in reader.read() {
        let block_mesh = mesh_query.get(event.block).ok();
        match block_mesh.and_then(|component| component.single_mesh.clone()) {
            Some(mesh) => {
                commands.spawn((
                    PhysicsObjectBundle {
                        velocity: Velocity::linear(event.initial_velocity),
                        collider: Collider::cuboid(HALF_SIDE, HALF_SIDE, HALF_SIDE),
                        ..default()
                    },
                    MaterialMeshBundle::<ArrayTextureMaterial> {
                        transform: Transform::from_translation(event.position),
                        mesh,
                        material: if block_mesh.unwrap().use_transparent_shader {
                            materials.transparent_material.clone().unwrap()
                        } else {
                            materials.opaque_material.clone().unwrap()
                        },
                        ..default()
                    },
                    FallingBlock(event.block, event.place_on_landing),
                ));
            }
            None => {
                commands.spawn((
                    PhysicsObjectBundle {
                        velocity: Velocity::linear(event.initial_velocity),
                        collider: Collider::cuboid(HALF_SIDE, HALF_SIDE, HALF_SIDE),
                        ..default()
                    },
                    PbrBundle {
                        transform: Transform::from_translation(event.position),
                        mesh: resources.default_mesh.clone(),
                        material: resources.default_material.clone(),
                        ..default()
                    },
                    FallingBlock(event.block, event.place_on_landing),
                ));
            }
        };
    }
}

fn falling_block_placer(
    level: Res<Level>,
    mut writer: EventWriter<LandedFallingBlockEvent>,
    block_query: Query<(Entity, &GlobalTransform, &FallingBlock)>,
    physics_query: Query<&BlockPhysics>,
) {
    for (entity, tf, falling_block) in block_query.iter() {
        let bottom_pos = tf.translation() - Vec3::new(0.0, 0.5, 0.0);
        if let Some(hit_entity) = level.get_block_entity(bottom_pos.into()) {
            if let Ok(physics) = physics_query.get(hit_entity) {
                match physics {
                    BlockPhysics::Empty => {}
                    BlockPhysics::Solid => writer.send(LandedFallingBlockEvent {
                        position: tf.translation().into(),
                        block: falling_block.0,
                        place_on_landing: falling_block.1,
                        faller: entity,
                    }),
                    BlockPhysics::BottomSlab(height) => writer.send(LandedFallingBlockEvent {
                        position: (tf.translation() + Vec3::new(0.0, 1.0 - height, 0.0)).into(),
                        block: falling_block.0,
                        place_on_landing: falling_block.1,
                        faller: entity,
                    }),
                }
            }
        }
    }
}

fn on_block_landed(
    mut reader: EventReader<LandedFallingBlockEvent>,
    level: Res<Level>,
    id_query: Query<&BlockId>,
    mut commands: Commands,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
) {
    for event in reader.read() {
        let mut exists = false;
        if let Some(mut ec) = commands.get_entity(event.faller) {
            exists = true;
            ec.despawn();
        }
        if exists && event.place_on_landing {
            level.set_block_entity(
                event.position,
                BlockType::Filled(event.block),
                &id_query,
                &mut update_writer,
                &mut commands,
            );
        }
    }
}
