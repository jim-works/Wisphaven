pub mod chunk;
mod level;

pub use level::*;

mod block_buffer;
pub use block_buffer::*;

mod block;
use bevy::prelude::*;
pub use block::*;
use serde::{Serialize, Deserialize};

mod atmosphere;

pub mod events;
pub mod settings;
pub mod blocks;

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
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum LevelLoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_set(PostUpdate, LevelSystemSet::PostUpdate.run_if(in_state(LevelLoadState::Loaded)))
            .configure_set(Update, LevelSystemSet::Main.run_if(in_state(LevelLoadState::Loaded)))
            .configure_set(Update, LevelSystemSet::Despawn.after(LevelSystemSet::Main).after(LevelSystemSet::LoadingAndMain))
            .configure_set(Update, LevelSystemSet::Despawn.run_if(in_state(LevelLoadState::Loaded)))
            .configure_set(Update, LevelSystemSet::LoadingAndMain.run_if(in_state(LevelLoadState::Loading).or_else(in_state(LevelLoadState::Loaded))))
            .add_plugins(atmosphere::AtmospherePlugin)
            .add_plugins(blocks::BlocksPlugin)
            .add_plugins(events::WorldEventsPlugin)
            .add_state::<LevelLoadState>()

            //needed for NamedBlockMesh
            .register_type::<[std::path::PathBuf; 6]>()
            .register_type::<BlockName>()
            .register_type::<UsableBlock>()
            .register_type::<NamedBlockMesh>()
            .register_type::<NamedBlockMeshShape>()
            .register_type::<BlockPhysics>()
        ;
    }
}

pub struct BlockcastHit {
    pub hit_pos: Vec3,
    pub block_pos: BlockCoord,
    pub block: BlockType,
    pub normal: BlockCoord,
}

//ids may not be stable across program runs. to get a specific id for an entity or name,
// use the corresponding registry. DO NOT HARDCODE (unless the backing id dict is hardcoded)
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Id {
    #[default]
    Empty,
    Basic(u32),
    Dynamic(u32)
}

impl Id {
    pub fn with_id(self, new_id: u32) -> Self {
        match self {
            Id::Empty => Id::Empty,
            Id::Basic(_) => Id::Basic(new_id),
            Id::Dynamic(_) => Id::Dynamic(new_id)
        }
    }
}