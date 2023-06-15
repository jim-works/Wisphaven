pub mod chunk;
mod level;
pub use level::*;

mod block_buffer;
pub use block_buffer::*;

mod block;
use bevy::prelude::*;
pub use block::*;

mod atmosphere;

pub mod events;
pub mod settings;


#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum LevelSystemSet {
    //systems in main should not despawn any entities, and don't have to worry about entity despawning
    Main,
    //all the despawning happens in the despawn set
    Despawn
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum LevelLoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded,
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(LevelSystemSet::Despawn.after(LevelSystemSet::Main))
            .add_plugin(atmosphere::AtmospherePlugin)
            .add_event::<events::CreateLevelEvent>()
            .add_event::<events::OpenLevelEvent>()
            .add_state::<LevelLoadState>()
        ;
    }
}

pub struct BlockcastHit {
    pub hit_pos: Vec3,
    pub block_pos: BlockCoord,
    pub block: BlockType,
    pub normal: BlockCoord,
}