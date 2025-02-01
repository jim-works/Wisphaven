pub mod chunk;
mod level;
pub mod util;

pub use level::*;

mod block_buffer;
pub use block_buffer::*;

mod block;
use ::util::plugin::UtilSystemSet;
use bevy::prelude::*;
pub use block::*;
use serde::{Deserialize, Serialize};

use crate::{physics::PhysicsSystemSet, GameState};

use self::chunk::ChunkCoord;

pub mod atmosphere;

pub mod effects;
pub mod events;
pub mod settings;
pub mod world_utils;

#[cfg(test)]
mod test;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum LevelSystemSet {
    //systems in main should not despawn any entities, and don't have to worry about entity despawning. only runs in LevelLoadState::Loaded
    Main,
    //all the despawning happens in the despawn set. only runs in LevelLoadState::Loaded
    Despawn,
    //Post-update, runs after both main and despawn, in LevelLoadState::Loaded
    PostUpdate,
    //like main, but also runs in only runs in LevelLoadState::Loading
    LoadingAndMain,
    //Update, runs after main/loading in main, in LevelLoadState::Loaded and Loading
    //system buffers from main and loading and main applied beforehand
    AfterLoadingAndMain,
    //fixedupdate
    PreTick,
    Tick,
    PostTick,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, SubStates, Default)]
#[source(GameState = GameState::Game)]
pub enum LevelLoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded,
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            LevelSystemSet::PostUpdate
                .run_if(in_state(LevelLoadState::Loaded))
                .run_if(in_state(GameState::Game)),
        )
        .configure_sets(
            Update,
            LevelSystemSet::AfterLoadingAndMain
                .run_if(in_state(LevelLoadState::Loading).or(in_state(LevelLoadState::Loaded)))
                .run_if(in_state(GameState::Game))
                .after(LevelSystemSet::LoadingAndMain)
                .after(LevelSystemSet::Main),
        )
        .configure_sets(
            Update,
            LevelSystemSet::Main
                .run_if(in_state(LevelLoadState::Loaded))
                .run_if(in_state(GameState::Game))
                .after(UtilSystemSet),
        )
        .configure_sets(
            Update,
            LevelSystemSet::Despawn
                .after(LevelSystemSet::Main)
                .after(LevelSystemSet::LoadingAndMain)
                .after(LevelSystemSet::AfterLoadingAndMain),
        )
        .configure_sets(
            Update,
            LevelSystemSet::LoadingAndMain
                .run_if(in_state(LevelLoadState::Loading).or(in_state(LevelLoadState::Loaded)))
                .run_if(in_state(GameState::Game))
                .after(UtilSystemSet),
        )
        .configure_sets(
            FixedUpdate,
            (
                LevelSystemSet::PreTick
                    .before(PhysicsSystemSet::Main)
                    .after(UtilSystemSet),
                LevelSystemSet::Tick.in_set(PhysicsSystemSet::Main),
                LevelSystemSet::PostTick.after(PhysicsSystemSet::UpdateDerivatives),
            )
                .chain()
                .run_if(in_state(LevelLoadState::Loaded))
                .run_if(in_state(GameState::Game)),
        )
        .add_systems(
            Update,
            apply_deferred
                .after(LevelSystemSet::Main)
                .after(LevelSystemSet::LoadingAndMain)
                .before(LevelSystemSet::AfterLoadingAndMain),
        )
        .add_systems(Update, check_chunk_boundary)
        .add_plugins((
            effects::EffectsPlugin,
            events::WorldEventsPlugin,
            util::LevelUtilsPlugin,
            atmosphere::AtmospherePlugin,
        ))
        .add_sub_state::<LevelLoadState>()
        .enable_state_scoped_entities::<LevelLoadState>()
        .add_event::<ChunkBoundaryCrossedEvent>()
        //needed for NamedBlockMesh
        .register_type::<[std::path::PathBuf; 6]>()
        .register_type::<[std::path::PathBuf; 2]>()
        .register_type::<BlockName>()
        .register_type::<UsableBlock>()
        .register_type::<BlockCoord>()
        .register_type::<NamedBlockMesh>()
        .register_type::<NamedBlockMeshShape>()
        .register_type::<BlockPhysics>();
    }
}

#[derive(Debug)]
pub struct BlockcastHit {
    pub hit_pos: Vec3,
    pub block_pos: BlockCoord,
    pub block: Option<BlockType>,
    pub normal: BlockCoord,
}

//ids may not be stable across program runs. to get a specific id for an entity or name,
// use the corresponding registry. DO NOT HARDCODE (unless the backing id dict is hardcoded)
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Id {
    #[default]
    Empty,
    Basic(u32),
    Dynamic(u32),
}

impl Id {
    pub fn with_id(self, new_id: u32) -> Self {
        match self {
            Id::Empty => Id::Empty,
            Id::Basic(_) => Id::Basic(new_id),
            Id::Dynamic(_) => Id::Dynamic(new_id),
        }
    }
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
