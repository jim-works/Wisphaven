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
        BlockType, Level,
    },
    worldgen::worldgen::GeneratedChunk,
};

use super::SPAWN_CHUNK_TIME_BUDGET_COUNT;

#[derive(Resource)]
pub struct GeneratePhysicsTimer {
    pub timer: Timer,
}

#[derive(Component)]
pub struct NeedsPhysics {}

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
    mut timer: ResMut<GeneratePhysicsTimer>,
    mut commands: Commands,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }
    let now = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    let mut len = 0;
    for (entity, coord) in query.iter() {
        if let Some(ctype) = level.get_chunk(*coord) {
            if let ChunkType::Full(chunk) = ctype.value() {
                let meshing = chunk.clone();
                len += 1;
                let task = pool.spawn(async move {
                    let mut data = PhysicsGenerationData {
                        colliders: Vec::new(),
                    };
                    gen_physics(&meshing, &mut data);
                    data
                });
                commands
                    .entity(entity)
                    .remove::<NeedsPhysics>()
                    .insert(GeneratePhysicsTask { task });
            }
        }
    }
    let duration = Instant::now().duration_since(now).as_millis();
    if len > 0 {
        println!(
            "queued physics generation for {} chunks in {}ms",
            len, duration
        );
    }
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
    //todo: parallelize this
    //(can't right now as Commands does not implement clone)
    let mut len = 0;
    let now = Instant::now();
    for (entity, opt_collider, tf, mut task) in query.iter_mut() {
        if let Some(data) = future::block_on(future::poll_once(&mut task.task)) {
            len += 1;

            if let Some(collider) = opt_collider {
                //remove old collider
                commands.entity(collider.child).despawn();
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
        println!("spawned {} chunk meshes in {}ms", len, duration);
    }
}
fn get_collider(origin: ChunkIdx) -> (Vec3, Quat, Collider) {
    (
        origin.get_block_center(),
        Quat::IDENTITY,
        //half-extents
        Collider::cuboid(0.5, 0.5, 0.5),
    )
}
fn gen_physics(chunk: &Chunk, data: &mut PhysicsGenerationData) {
    let mut compound = Vec::new();
    for i in 0..chunk::BLOCKS_PER_CHUNK {
        let coord = ChunkIdx::from_usize(i);
        //TODO: greedy meshing for basic blocks
        let b = chunk[i];
        if let BlockType::Empty = b {
            continue;
        }
        //on edge, generate collider
        if coord.x == CHUNK_SIZE_U8 - 1
            || coord.y == CHUNK_SIZE_U8 - 1
            || coord.z == CHUNK_SIZE_U8 - 1
            || coord.x == 0
            || coord.y == 0
            || coord.z == 0
        {
            compound.push(get_collider(coord))
        } else if matches!(
            chunk[ChunkIdx::new(coord.x, coord.y, coord.z + 1)],
            BlockType::Empty
        ) || matches!(
            chunk[ChunkIdx::new(coord.x, coord.y + 1, coord.z)],
            BlockType::Empty
        ) || matches!(
            chunk[ChunkIdx::new(coord.x + 1, coord.y, coord.z)],
            BlockType::Empty
        ) || matches!(
            chunk[ChunkIdx::new(coord.x, coord.y, coord.z - 1)],
            BlockType::Empty
        ) || matches!(
            chunk[ChunkIdx::new(coord.x, coord.y - 1, coord.z)],
            BlockType::Empty
        ) || matches!(
            chunk[ChunkIdx::new(coord.x - 1, coord.y, coord.z)],
            BlockType::Empty
        ) {
            //has at least one air neighbor, generate collider
            compound.push(get_collider(coord))
        }
    }
    if compound.len() > 0 {
        data.colliders.push(Collider::compound(compound));
    }
}
