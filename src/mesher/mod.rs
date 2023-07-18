mod generator;
pub use generator::*;

mod mesh_lod;

pub mod materials;
pub use materials::{ChunkMaterial, ArrayTextureMaterial};

use bevy::{
    pbr::*,
    prelude::*,
};

use crate::world::LevelSystemSet;

use self::mesh_lod::LODMeshTimer;

pub struct MesherPlugin;

const SPAWN_MESH_TIME_BUDGET_COUNT: u32 = 1000;

impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {

        app.add_plugins(MaterialPlugin::<ArrayTextureMaterial>::default())
            .insert_resource(MeshTimer {
                timer: Timer::from_seconds(0.05, TimerMode::Repeating),
            })
            .insert_resource(LODMeshTimer {
                timer: Timer::from_seconds(0.05, TimerMode::Repeating),
            })
            .add_systems(
                Update,
                (
                    generator::poll_mesh_queue,
                    generator::queue_meshing,
                    mesh_lod::queue_meshing_lod,
                )
                    .in_set(LevelSystemSet::LoadingAndMain))
            .add_systems(Startup, materials::init)
            //can't be a startup system since init starts loading the chunk image asynchronously
            .add_systems(PreUpdate, materials::create_chunk_material);
    }
}

#[derive(Resource)]
pub struct TerrainTexture(pub Vec<Handle<Image>>);


