mod generator;
pub use generator::*;

pub mod extended_materials;
pub mod item_mesher;
pub mod materials;
pub mod order;
pub use materials::ChunkMaterial;

use bevy::{asset::load_internal_asset, pbr::*, prelude::*};

use crate::{
    mesher::extended_materials::TextureArrayExtension,
    serialization::state::GameLoadState,
    world::{
        chunk::{ChunkCoord, ChunkType},
        Level, LevelSystemSet,
    },
};

pub struct MesherPlugin;

const SPAWN_MESH_TIME_BUDGET_COUNT: u32 = 1000;

impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(item_mesher::ItemMesherPlugin);
        load_internal_asset!(
            app,
            Handle::weak_from_u128(21908015359337029746),
            "../../../../assets/shaders/texture_array.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, TextureArrayExtension>,
        > {
            prepass_enabled: false,
            ..default()
        })
        .add_systems(
            Update,
            (
                generator::poll_mesh_queue,
                generator::queue_meshing,
                order::set_meshing_order,
            )
                .in_set(LevelSystemSet::AfterLoadingAndMain),
        )
        .add_systems(Startup, materials::init)
        //can't be a startup system since init starts loading the chunk image asynchronously
        .add_systems(
            PreUpdate,
            materials::create_chunk_material.run_if(in_state(GameLoadState::Done)),
        );
    }
}

#[derive(Resource)]
pub struct TerrainTexture(pub Vec<Handle<Image>>);

pub fn is_chunk_ready_for_meshing(coord: ChunkCoord, level: &Level) -> bool {
    //i wish i could extrac this if let Some() shit into a function
    //but that makes the borrow checker angry
    for dx in -1..2 {
        for dy in -1..2 {
            for dz in -1..2 {
                let offset = ChunkCoord::new(dx, dy, dz);
                if offset.x == 0 && offset.y == 0 && offset.z == 0 {
                    continue; //don't check ourselves
                }
                if let Some(ctype) = level.get_chunk(coord + offset) {
                    if !matches!(ctype.value(), ChunkType::Full(_)) {
                        //chunk not ready
                        return false;
                    }
                } else {
                    //chunk not loaded
                    return false;
                }
            }
        }
    }
    //all neighboring chunks are ready!
    true
}
