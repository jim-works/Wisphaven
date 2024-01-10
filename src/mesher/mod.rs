mod generator;
pub use generator::*;

pub mod materials;
pub mod extended_materials;
pub use materials::ChunkMaterial;

use bevy::{
    pbr::*,
    prelude::*, asset::load_internal_asset,
};

use crate::{world::LevelSystemSet, serialization::state::GameLoadState, mesher::extended_materials::TextureArrayExtension};

pub struct MesherPlugin;

const SPAWN_MESH_TIME_BUDGET_COUNT: u32 = 1000;

impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            Handle::weak_from_u128(21908015359337029746),
            "../../assets/shaders/texture_array.wgsl",
            Shader::from_wgsl
        );
        app
            .add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, TextureArrayExtension>> {prepass_enabled: false, ..default()})
            .add_systems(
                Update,
                (
                    generator::poll_mesh_queue,
                    generator::queue_meshing,
                )
                    .in_set(LevelSystemSet::AfterLoadingAndMain))
            .add_systems(Startup, materials::init)
            //can't be a startup system since init starts loading the chunk image asynchronously
            .add_systems(PreUpdate, materials::create_chunk_material.run_if(in_state(GameLoadState::Done)));
    }
}

#[derive(Resource)]
pub struct TerrainTexture(pub Vec<Handle<Image>>);


