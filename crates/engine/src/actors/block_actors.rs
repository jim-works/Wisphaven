use bevy::prelude::*;

use util::direction::DirectionFlags;

use crate::{
    mesher::ChunkMaterial,
    physics::{
        collision::{Aabb, CollidingDirections},
        movement::Velocity,
        PhysicsBundle,
    },
    world::{
        events::ChunkUpdatedEvent, BlockCoord, BlockId, BlockMesh, BlockPhysics, BlockType, Level,
        LevelLoadState, LevelSystemSet,
    },
};

const HALF_SIDE: f32 = 0.45;
const MAX_PLANAR_VELOCITY: f32 = 0.25;

pub struct BlockActorPlugin;

impl Plugin for BlockActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnFallingBlockEvent>()
            .add_event::<LandedFallingBlockEvent>()
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
    pub falling_block: FallingBlock,
}

#[derive(Event)]
pub struct LandedFallingBlockEvent {
    pub position: BlockCoord,
    pub faller: Entity,
    pub falling_block: FallingBlock,
}

#[derive(Component, Clone, Copy)]
pub struct FallingBlock {
    pub block: Entity,
    pub place_on_landing: bool,
    pub impact_direcitons: DirectionFlags,
}

fn falling_block_spawner(
    mut reader: EventReader<SpawnFallingBlockEvent>,
    mut commands: Commands,
    mesh_query: Query<(&BlockMesh, Option<&BlockPhysics>)>,
    materials: Res<ChunkMaterial>,
) {
    const COLLIDER_SQUISH_FACTOR: f32 = 0.9; //squish the collider a bit so the collider can fall down 1x1 tunnels
    for event in reader.read() {
        if let Ok((block_mesh, opt_physics)) = mesh_query.get(event.falling_block.block) {
            if let Some(collider) = Aabb::from_block(opt_physics.unwrap_or(&BlockPhysics::Solid)) {
                if let Some(mesh) = block_mesh.single_mesh.clone() {
                    commands.spawn((
                        StateScoped(LevelLoadState::Loaded),
                        PhysicsBundle {
                            velocity: Velocity(event.initial_velocity),
                            collider: collider.scale(Vec3::ONE * COLLIDER_SQUISH_FACTOR),
                            ..default()
                        },
                        Transform::from_translation(event.position),
                        Mesh3d(mesh),
                        MeshMaterial3d(if block_mesh.use_transparent_shader {
                            materials.transparent_material.clone().unwrap()
                        } else {
                            materials.opaque_material.clone().unwrap()
                        }),
                        event.falling_block,
                    ));
                }
            }
        }
    }
}

fn falling_block_placer(
    level: Res<Level>,
    mut writer: EventWriter<LandedFallingBlockEvent>,
    block_query: Query<(
        Entity,
        &CollidingDirections,
        &FallingBlock,
        &Velocity,
        &Transform,
    )>,
) {
    const BACKTRACK_DIST: f32 = 1.0; //amount to look backward to find a suitable place for the block
    for (entity, hit, falling_block, v, tf) in block_query.iter() {
        if hit.0.intersects(falling_block.impact_direcitons) {
            //we had a collision on one of the allowed axes
            if let Some(placing_coord) = level.blockcast(
                tf.translation,
                -v.0.normalize_or_zero() * BACKTRACK_DIST,
                |opt_b| opt_b.map(|b| matches!(b, BlockType::Empty)).unwrap_or(true),
            ) {
                info!("hit {:?}", placing_coord);
                writer.send(LandedFallingBlockEvent {
                    position: placing_coord.block_pos,
                    faller: entity,
                    falling_block: *falling_block,
                });
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
        if exists && event.falling_block.place_on_landing {
            level.set_block_entity(
                event.position,
                BlockType::Filled(event.falling_block.block),
                &id_query,
                &mut update_writer,
                &mut commands,
            );
        }
    }
}
