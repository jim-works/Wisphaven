#![feature(let_chains)]
pub mod atmosphere;
pub mod block;
pub mod block_buffer;
pub mod chunk;
pub mod chunk_loading;
pub mod effects;
pub mod events;
pub mod level;
pub mod mesher;
pub mod settings;
pub mod spawn_point;
pub mod util;
pub mod worldgen;

use bevy::prelude::*;
use block::*;
use interfaces::scheduling::*;

use self::chunk::ChunkCoord;

#[cfg(test)]
mod test;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, check_chunk_boundary)
            .add_plugins((
                effects::EffectsPlugin,
                events::WorldEventsPlugin,
                util::LevelUtilsPlugin,
                atmosphere::AtmospherePlugin,
                chunk_loading::ChunkLoaderPlugin,
                mesher::MesherPlugin,
                worldgen::WorldGenPlugin,
                spawn_point::SpawnPointPlugin,
            ))
            .insert_resource(FixedUpdateBlockGizmos::default())
            .add_event::<ChunkBoundaryCrossedEvent>()
            //needed for NamedBlockMesh
            .register_type::<[std::path::PathBuf; 6]>()
            .register_type::<[std::path::PathBuf; 2]>()
            .register_type::<BlockName>()
            .register_type::<UsableBlock>()
            .register_type::<BlockCoord>()
            .register_type::<NamedBlockMesh>()
            .register_type::<NamedBlockMeshShape>();
    }
}

#[derive(Debug)]
pub struct BlockcastHit {
    pub hit_pos: Vec3,
    pub block_pos: BlockCoord,
    pub block: Option<BlockType>,
    pub normal: BlockCoord,
}

//will send a ChunkBoundaryCrossedEvent whenever this entity crosses a chunk boundary
//(requires globaltransform)
#[derive(Component)]
pub struct ChunkBoundaryNotifier {
    pub last_position: ChunkCoord,
}

#[derive(Event)]
pub struct ChunkBoundaryCrossedEvent {
    pub entity: Entity,
    pub old_position: ChunkCoord,
    pub new_position: ChunkCoord,
}

fn check_chunk_boundary(
    mut notifiers: Query<(Entity, &mut ChunkBoundaryNotifier, &GlobalTransform)>,
    mut writer: EventWriter<ChunkBoundaryCrossedEvent>,
) {
    for (entity, mut notifier, tf) in notifiers.iter_mut() {
        let new_position: ChunkCoord = tf.translation().into();
        if notifier.last_position != new_position {
            writer.send(ChunkBoundaryCrossedEvent {
                entity,
                old_position: notifier.last_position,
                new_position,
            });
            notifier.last_position = new_position;
        }
    }
}

#[derive(Resource, Default)]
pub struct FixedUpdateBlockGizmos {
    pub blocks: ahash::HashSet<BlockCoord>,
}
