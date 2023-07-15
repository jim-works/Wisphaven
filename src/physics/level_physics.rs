use std::time::Instant;

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_rapier3d::prelude::*;
use futures_lite::future;

use crate::{
    world::{
        chunk::{self, *},
        Level, BlockPhysics,
    },
    worldgen::GeneratedChunk,
};

use super::SPAWN_CHUNK_TIME_BUDGET_COUNT;

#[derive(Resource)]
pub struct GeneratePhysicsTimer {
    pub timer: Timer,
}

#[derive(Component)]
pub struct NeedsPhysics;

#[derive(Component)]
pub struct ChunkColliderGenerated {
    child: Entity,
}

#[derive(Component)]
pub struct GeneratePhysicsTask {
    pub task: Task<PhysicsGenerationData>,
}

pub struct PhysicsGenerationData {
    pub colliders: Vec<Collider>,
}

pub fn queue_gen_physics(
    query: Query<(Entity, &ChunkCoord), (With<GeneratedChunk>, With<NeedsPhysics>)>,
    level: Res<Level>,
    time: Res<Time>,
    block_query: Query<&BlockPhysics>,
    mut timer: ResMut<GeneratePhysicsTimer>,
    commands: ParallelCommands,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }
    let pool = AsyncComputeTaskPool::get();
    query.par_iter().for_each(|(entity, coord)| {
        if let Some(ctype) = level.get_chunk(*coord) {
            if let ChunkType::Full(chunk) = ctype.value() {
                let meshing = chunk.with_storage(Box::new(chunk.blocks.get_components(&block_query)));
                let task = pool.spawn(async move {
                    let mut data = PhysicsGenerationData {
                        colliders: Vec::new(),
                    };
                    gen_physics(&meshing, &mut data);
                    data
                });
                commands.command_scope(|mut commands| {
                    commands.entity(entity)
                    .remove::<NeedsPhysics>()
                    .insert(GeneratePhysicsTask { task });
                });
            }
        }
    });
}

pub fn poll_gen_physics_queue(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        Option<&ChunkColliderGenerated>,
        &GlobalTransform,
        &mut GeneratePhysicsTask,
    )>,
) {
    let mut len = 0;
    let now = Instant::now();
    for (entity, opt_collider, tf, mut task) in query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            len += 1;

            if let Some(collider) = opt_collider {
                //remove old collider
                commands.entity(collider.child).despawn_recursive();
            }
            //add new collider
            let mut child_commands = commands.spawn(*tf);
            let child = child_commands.id();

            for collider in data.colliders {
                child_commands.insert(collider);
            }

            commands.entity(entity).add_child(child);
            commands
                .entity(entity)
                .remove::<GeneratePhysicsTask>()
                .insert(ChunkColliderGenerated { child });
            if len > SPAWN_CHUNK_TIME_BUDGET_COUNT {
                break;
            }
        }
    }
    let duration = Instant::now().duration_since(now).as_millis();
    if len > 0 {
        debug!("spawned {} chunk meshes in {}ms", len, duration);
    }
}
fn get_collider(block: &BlockPhysics, origin: ChunkIdx) -> Option<(Vec3, Quat, Collider)> {
    match block {
        BlockPhysics::Solid => Some((
            origin.get_block_center(),
            Quat::IDENTITY,
            //half-extents
            Collider::cuboid(0.5, 0.5, 0.5),
        )),
        BlockPhysics::BottomSlab(height) => Some((
            origin.get_block_center()-Vec3::new(0.0,0.5-height*0.5,0.0),
            Quat::IDENTITY,
            //half-extents
            Collider::cuboid(0.5, 0.5*height, 0.5),
        )),
        BlockPhysics::Empty => None
    }
}
fn gen_physics<T: ChunkStorage<BlockPhysics>>(chunk: &Chunk<T,BlockPhysics>, data: &mut PhysicsGenerationData) {
    let mut compound = Vec::new();
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        let coord = ChunkIdx::from_usize(i);
        //TODO: greedy meshing
        let b = &chunk[i];
        if matches!(b, BlockPhysics::Empty) {
            continue;
        }
        //on edge or has air neighbor, generate collider
        if coord.x == CHUNK_SIZE_U8 - 1
            || coord.y == CHUNK_SIZE_U8 - 1
            || coord.z == CHUNK_SIZE_U8 - 1
            || coord.x == 0
            || coord.y == 0
            || coord.z == 0
            || chunk[ChunkIdx::new(coord.x, coord.y, coord.z + 1)].has_hole(crate::util::Direction::NegZ)
            || chunk[ChunkIdx::new(coord.x, coord.y, coord.z - 1)].has_hole(crate::util::Direction::PosZ)
            || chunk[ChunkIdx::new(coord.x, coord.y+1, coord.z)].has_hole(crate::util::Direction::NegY)
            || chunk[ChunkIdx::new(coord.x, coord.y-1, coord.z)].has_hole(crate::util::Direction::PosY)
            || chunk[ChunkIdx::new(coord.x+1, coord.y, coord.z)].has_hole(crate::util::Direction::NegX)
            || chunk[ChunkIdx::new(coord.x-1, coord.y, coord.z)].has_hole(crate::util::Direction::PosX)
        {
            if let Some(col) = get_collider(&b, coord) {
                compound.push(col);
            }
        }
    }
    if !compound.is_empty() {
        data.colliders.push(Collider::compound(compound));
    }
}
