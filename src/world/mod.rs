pub mod chunk;
mod level;
use std::sync::OnceLock;

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
    //systems in main should not despawn any entities, and don't have to worry about entity despawning. only runs in LevelLoadState::Loaded
    Main,
    //all the despawning happens in the despawn set. only runs in LevelLoadState::Loaded
    Despawn,
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
            .configure_set(LevelSystemSet::Main.in_set(OnUpdate(LevelLoadState::Loaded)))
            .configure_set(LevelSystemSet::Despawn.after(LevelSystemSet::Main).after(LevelSystemSet::LoadingAndMain))
            .configure_set(LevelSystemSet::Despawn.in_set(OnUpdate(LevelLoadState::Loaded)))
            .configure_set(LevelSystemSet::LoadingAndMain.run_if(in_state(LevelLoadState::Loading).or_else(in_state(LevelLoadState::Loaded))))
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

// pub static BLOCK_REGISTRY: BlockRegistry = BlockRegistry {
//     data: [BlockMesh::MultiTexture([1,0,1,1,3,1]),
//         BlockMesh::Uniform(3),
//         BlockMesh::MultiTexture([5,6,5,5,6,5]),
//         BlockMesh::Uniform(7)
//         ]
// };

pub fn get_block_registry() -> &'static BlockRegistry {
    static REGISTRY: OnceLock<BlockRegistry> = OnceLock::new();
    REGISTRY.get_or_init(initialize_block_registry)
}

fn initialize_block_registry() -> BlockRegistry {
    let mut registry = BlockRegistry{ meshes: Vec::new() };
    //grass - 0
    registry.meshes.push(BlockMesh::MultiTexture([1,0,1,1,3,1]));
    //dirt - 1
    registry.meshes.push(BlockMesh::Uniform(3));
    //stone - 2
    registry.meshes.push(BlockMesh::Uniform(2));
    //log - 3
    registry.meshes.push(BlockMesh::MultiTexture([5,6,5,5,6,5]));
    //leaves - 4
    registry.meshes.push(BlockMesh::Uniform(7));
    //log slab - 5
    registry.meshes.push(BlockMesh::BottomSlab(0.5, [5,6,5,5,6,5]));

    registry
}