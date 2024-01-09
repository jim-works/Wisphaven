mod generator;
pub use generator::*;

pub mod materials;
pub use materials::{ChunkMaterial, ArrayTextureMaterial};

use bevy::{
    pbr::*,
    prelude::*, asset::load_internal_asset,
};

use crate::{world::LevelSystemSet, serialization::state::GameLoadState};

pub struct MesherPlugin;

const SPAWN_MESH_TIME_BUDGET_COUNT: u32 = 1000;

impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            Handle::weak_from_u128(21908015359337029744),
            "../../assets/shaders/array_texture_io.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            Handle::weak_from_u128(21908015359337029745),
            "../../assets/shaders/array_texture_input.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<ArrayTextureMaterial> {prepass_enabled: false, ..default()})
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


