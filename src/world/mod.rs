pub mod chunk;
mod level;
pub use level::*;

mod block_buffer;
pub use block_buffer::*;

mod block;
use bevy::prelude::*;
pub use block::*;

mod atmosphere;


#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum LevelSystemSet {
    //systems in main should not despawn any entities, and don't have to worry about entity despawning
    Main,
    //all the despawning happens in the despawn set
    Despawn
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(LevelSystemSet::Despawn.after(LevelSystemSet::Main))
            .add_plugin(atmosphere::AtmospherePlugin)
        ;
    }
}

pub struct BlockcastHit {
    pub hit_pos: Vec3,
    pub block_pos: BlockCoord,
    pub block: BlockType,
    pub normal: BlockCoord,
}