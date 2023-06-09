mod mesher;
pub use mesher::*;

mod mesh_lod;

use bevy::{prelude::*, render::{render_resource::{AsBindGroup, ShaderRef, VertexFormat, RenderPipelineDescriptor, SpecializedMeshPipelineError}, texture::ImageSampler, mesh::{MeshVertexAttribute, MeshVertexBufferLayout}}, reflect::TypeUuid, asset::LoadState, pbr::*};

use crate::world::{LevelSystemSet, chunk};

pub struct MesherPlugin;

const SPAWN_MESH_TIME_BUDGET_COUNT: u32 = 1000;


impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(MaterialPlugin::<ArrayTextureMaterial>::default())
            .insert_resource(MeshTimer{timer: Timer::from_seconds(0.05, TimerMode::Repeating)})
            .add_systems((mesher::poll_mesh_queue,mesher::queue_meshing,mesh_lod::queue_meshing_lod).in_set(LevelSystemSet::Main))
            .add_startup_system(init)
            .add_system(create_chunk_material)
            ;
    }
}

#[derive(Resource)]
pub struct ChunkMaterial {
    tex_handle: Handle<Image>,
    pub opaque_material: Option<Handle<ArrayTextureMaterial>>,
    pub transparent_material: Option<Handle<ArrayTextureMaterial>>,
    pub layers: u32,
    pub loaded: bool
}

#[derive(AsBindGroup, Debug, Clone, TypeUuid)]
//https://www.uuidtools.com/generate/v4
#[uuid="c275fe2c-7500-46b2-a43d-e3ec8a76f4e4"]
//TODO: look at standard material source code to get pbr working\
//for layer
//https://github.com/bevyengine/bevy/blob/main/examples/shader/custom_vertex_attribute.rs
pub struct ArrayTextureMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub array_texture: Handle<Image>,
    pub alpha_mode: AlphaMode,
    
}

//random high id to not conflict
const ATTRIBUTE_ARRAYTEXTURE_LAYER: MeshVertexAttribute = MeshVertexAttribute::new("BlendColor", 970540917, VertexFormat::Uint32);

impl Material for ArrayTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/array_texture.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.get_layout(&[
            //TODO: fix this for pbr and shading in general
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_ARRAYTEXTURE_LAYER.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

fn init(
    mut commands: Commands,
    assets: Res<AssetServer>
) {
    commands.insert_resource(ChunkMaterial {
        tex_handle: assets.load("textures/tileset.png"),
        opaque_material: None,
        transparent_material: None,
        layers: 0,
        loaded: false
    });
}

fn create_chunk_material (
    assets: Res<AssetServer>,
    mut chunk_material: ResMut<ChunkMaterial>,
    mut materials: ResMut<Assets<ArrayTextureMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    if matches!(chunk_material.opaque_material, Some(_)) || assets.get_load_state(chunk_material.tex_handle.clone()) != LoadState::Loaded
    {
        return;
    }
    let image = images.get_mut(&chunk_material.tex_handle).unwrap();
    //set filtering for clean pixel art
    image.sampler_descriptor = ImageSampler::nearest();

    // Create a new array texture asset from the loaded texture.
    let array_layers = 4;
    image.reinterpret_stacked_2d_as_array(array_layers);
    chunk_material.opaque_material = Some(materials.add(ArrayTextureMaterial {
        array_texture: chunk_material.tex_handle.clone(),
        alpha_mode: AlphaMode::Opaque
    }));
    chunk_material.transparent_material = Some(materials.add(ArrayTextureMaterial {
        array_texture: chunk_material.tex_handle.clone(),
        alpha_mode: AlphaMode::Blend
    }));  
    chunk_material.layers = array_layers;
    chunk_material.loaded = true;
    info!("Loaded chunk material");
}