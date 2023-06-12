mod mesher;
pub use mesher::*;

mod mesh_lod;

use bevy::{
    asset::LoadState,
    pbr::*,
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayout},
        render_resource::{
            AddressMode, AsBindGroup, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError, VertexFormat, Face, TextureFormat, AsBindGroupShaderType, ShaderType,
        },
        texture::ImageSampler, render_asset::RenderAssets,
    },
};

use crate::world::{chunk, LevelSystemSet};

pub struct MesherPlugin;

const SPAWN_MESH_TIME_BUDGET_COUNT: u32 = 1000;

impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MaterialPlugin::<ArrayTextureMaterial>::default())
            .insert_resource(MeshTimer {
                timer: Timer::from_seconds(0.05, TimerMode::Repeating),
            })
            .add_systems(
                (
                    mesher::poll_mesh_queue,
                    mesher::queue_meshing,
                    mesh_lod::queue_meshing_lod,
                )
                    .in_set(LevelSystemSet::Main),
            )
            .add_startup_system(init)
            .add_system(create_chunk_material);
    }
}

#[derive(Resource)]
pub struct ChunkMaterial {
    tex_handle: Handle<Image>,
    pub opaque_material: Option<Handle<ArrayTextureMaterial>>,
    pub transparent_material: Option<Handle<ArrayTextureMaterial>>,
    pub layers: u32,
    pub loaded: bool,
}

/// The GPU representation of the uniform data of a [`StandardMaterial`].
#[derive(Clone, Default, ShaderType)]
pub struct ArrayTextureMaterialUniform {
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between.
    pub base_color: Vec4,
    // Use a color for user friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    pub emissive: Vec4,
    /// Linear perceptual roughness, clamped to [0.089, 1.0] in the shader
    /// Defaults to minimum of 0.089
    pub roughness: f32,
    /// From [0.0, 1.0], dielectric to pure metallic
    pub metallic: f32,
    /// Specular intensity for non-metals on a linear scale of [0.0, 1.0]
    /// defaults to 0.5 which is mapped to 4% reflectance in the shader
    pub reflectance: f32,
    /// The [`StandardMaterialFlags`] accessible in the `wgsl` shader.
    pub flags: u32,
    /// When the alpha mode mask flag is set, any base color alpha above this cutoff means fully opaque,
    /// and any below means fully transparent.
    pub alpha_cutoff: f32,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ArrayTextureMaterialKey {
    normal_map: bool,
    cull_mode: Option<Face>,
    depth_bias: i32,
}

#[derive(AsBindGroup, Reflect, FromReflect, Debug, Clone, TypeUuid)]
//https://www.uuidtools.com/generate/v4
#[uuid = "c275fe2c-7500-46b2-a43d-e3ec8a76f4e4"]
//taken from bevy's StandardMaterial (https://github.com/bevyengine/bevy/blob/a1494e53dfa90e56bd14452b1efb930bd3e79821/crates/bevy_pbr/src/pbr_material.rs#L23)
#[bind_group_data(ArrayTextureMaterialKey)]
#[uniform(0, ArrayTextureMaterialUniform)]
pub struct ArrayTextureMaterial {
    pub base_color: Color,

    //changed to dimension="2d_array"
    #[texture(1, dimension = "2d_array")]
    #[sampler(2)]
    pub base_color_texture: Option<Handle<Image>>,

    pub emissive: Color,
    // pub test: StandardMaterial,

    #[texture(3)]
    #[sampler(4)]
    pub emissive_texture: Option<Handle<Image>>,

    pub perceptual_roughness: f32,

    pub metallic: f32,

    #[texture(5)]
    #[sampler(6)]
    pub metallic_roughness_texture: Option<Handle<Image>>,

    #[doc(alias = "specular_intensity")]
    pub reflectance: f32,

    #[texture(9)]
    #[sampler(10)]
    pub normal_map_texture: Option<Handle<Image>>,

    pub flip_normal_map_y: bool,

    #[texture(7)]
    #[sampler(8)]
    pub occlusion_texture: Option<Handle<Image>>,
    pub double_sided: bool,

    #[reflect(ignore)]
    pub cull_mode: Option<Face>,
    pub unlit: bool,

    pub fog_enabled: bool,

    pub alpha_mode: AlphaMode,

    pub depth_bias: f32,

    #[texture(11)]
    #[sampler(12)]
    pub depth_map: Option<Handle<Image>>,
    //removed by me
    // pub parallax_depth_scale: f32,

    // pub parallax_mapping_method: ParallaxMappingMethod,

    // pub max_parallax_layer_count: f32,
}

impl From<&ArrayTextureMaterial> for ArrayTextureMaterialKey {
    fn from(material: &ArrayTextureMaterial) -> Self {
        ArrayTextureMaterialKey {
            normal_map: material.normal_map_texture.is_some(),
            cull_mode: material.cull_mode,
            depth_bias: material.depth_bias as i32,
        }
    }
}

impl AsBindGroupShaderType<ArrayTextureMaterialUniform> for ArrayTextureMaterial {
    fn as_bind_group_shader_type(&self, images: &RenderAssets<Image>) -> ArrayTextureMaterialUniform {
        let mut flags = StandardMaterialFlags::NONE;
        if self.base_color_texture.is_some() {
            flags |= StandardMaterialFlags::BASE_COLOR_TEXTURE;
        }
        if self.emissive_texture.is_some() {
            flags |= StandardMaterialFlags::EMISSIVE_TEXTURE;
        }
        if self.metallic_roughness_texture.is_some() {
            flags |= StandardMaterialFlags::METALLIC_ROUGHNESS_TEXTURE;
        }
        if self.occlusion_texture.is_some() {
            flags |= StandardMaterialFlags::OCCLUSION_TEXTURE;
        }
        if self.double_sided {
            flags |= StandardMaterialFlags::DOUBLE_SIDED;
        }
        if self.unlit {
            flags |= StandardMaterialFlags::UNLIT;
        }
        if self.fog_enabled {
            flags |= StandardMaterialFlags::FOG_ENABLED;
        }
        let has_normal_map = self.normal_map_texture.is_some();
        if has_normal_map {
            if let Some(texture) = images.get(self.normal_map_texture.as_ref().unwrap()) {
                match texture.texture_format {
                    // All 2-component unorm formats
                    TextureFormat::Rg8Unorm
                    | TextureFormat::Rg16Unorm
                    | TextureFormat::Bc5RgUnorm
                    | TextureFormat::EacRg11Unorm => {
                        flags |= StandardMaterialFlags::TWO_COMPONENT_NORMAL_MAP;
                    }
                    _ => {}
                }
            }
            if self.flip_normal_map_y {
                flags |= StandardMaterialFlags::FLIP_NORMAL_MAP_Y;
            }
        }
        // NOTE: 0.5 is from the glTF default - do we want this?
        let mut alpha_cutoff = 0.5;
        match self.alpha_mode {
            AlphaMode::Opaque => flags |= StandardMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode::Mask(c) => {
                alpha_cutoff = c;
                flags |= StandardMaterialFlags::ALPHA_MODE_MASK;
            }
            AlphaMode::Blend => flags |= StandardMaterialFlags::ALPHA_MODE_BLEND,
            AlphaMode::Premultiplied => flags |= StandardMaterialFlags::ALPHA_MODE_PREMULTIPLIED,
            AlphaMode::Add => flags |= StandardMaterialFlags::ALPHA_MODE_ADD,
            AlphaMode::Multiply => flags |= StandardMaterialFlags::ALPHA_MODE_MULTIPLY,
        };

        ArrayTextureMaterialUniform {
            base_color: self.base_color.as_linear_rgba_f32().into(),
            emissive: self.emissive.as_linear_rgba_f32().into(),
            roughness: self.perceptual_roughness,
            metallic: self.metallic,
            reflectance: self.reflectance,
            flags: flags.bits(),
            alpha_cutoff,
        }
    }
}

//random high id to not conflict
//would make more sense to be u32, but the texture sampler in the shader doesn't like u32 for some reason
const ATTRIBUTE_ARRAYTEXTURE_LAYER: MeshVertexAttribute =
    MeshVertexAttribute::new("TexLayer", 970540917, VertexFormat::Sint32);

impl Material for ArrayTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/array_texture.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/array_texture.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
    fn prepass_fragment_shader() -> ShaderRef {
        PBR_PREPASS_SHADER_HANDLE.typed().into()
    }
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.get_layout(&[
            //standard bevy pbr stuff (check assets/shaders/array_texture.wgsl Vertex struct)
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            // Mesh::ATTRIBUTE_TANGENT.at_shader_location(3),
            // Mesh::ATTRIBUTE_COLOR.at_shader_location(4),
            // Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(5),
            // Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(6),
            //my addition
            ATTRIBUTE_ARRAYTEXTURE_LAYER.at_shader_location(7),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

impl Default for ArrayTextureMaterial {
    fn default() -> Self {
        //taken from bevy's StandardMaterial
        ArrayTextureMaterial {
            // White because it gets multiplied with texture values if someone uses
            // a texture.
            base_color: Color::rgb(1.0, 1.0, 1.0),
            base_color_texture: None,
            emissive: Color::BLACK,
            emissive_texture: None,
            // Matches Blender's default roughness.
            perceptual_roughness: 0.5,
            // Metallic should generally be set to 0.0 or 1.0.
            metallic: 0.0,
            metallic_roughness_texture: None,
            // Minimum real-world reflectance is 2%, most materials between 2-5%
            // Expressed in a linear scale and equivalent to 4% reflectance see
            // <https://google.github.io/filament/Material%20Properties.pdf>
            reflectance: 0.02,
            occlusion_texture: None,
            normal_map_texture: None,
            flip_normal_map_y: false,
            double_sided: false,
            cull_mode: Some(Face::Back),
            unlit: false,
            fog_enabled: true,
            alpha_mode: AlphaMode::Opaque,
            depth_bias: 0.0,
            depth_map: None,
            // parallax_depth_scale: 0.1,
            // max_parallax_layer_count: 16.0,
            // parallax_mapping_method: ParallaxMappingMethod::Occlusion,
        }
    }
}

fn init(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ChunkMaterial {
        tex_handle: assets.load("textures/tileset.png"),
        opaque_material: None,
        transparent_material: None,
        layers: 0,
        loaded: false,
    });
}

fn create_chunk_material(
    assets: Res<AssetServer>,
    mut chunk_material: ResMut<ChunkMaterial>,
    mut materials: ResMut<Assets<ArrayTextureMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    if matches!(chunk_material.opaque_material, Some(_))
        || assets.get_load_state(chunk_material.tex_handle.clone()) != LoadState::Loaded
    {
        return;
    }
    let image = images.get_mut(&chunk_material.tex_handle).unwrap();
    //set filtering for clean pixel art, repeat textures for greedy meshing
    image.sampler_descriptor =
        ImageSampler::Descriptor(bevy::render::render_resource::SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            ..ImageSampler::nearest_descriptor()
        });

    // Create a new array texture asset from the loaded texture.
    let array_layers = 4;
    image.reinterpret_stacked_2d_as_array(array_layers);
    chunk_material.opaque_material = Some(materials.add(ArrayTextureMaterial {
        base_color_texture: Some(chunk_material.tex_handle.clone()),
        alpha_mode: AlphaMode::Opaque,
        ..default()
    }));
    chunk_material.transparent_material = Some(materials.add(ArrayTextureMaterial {
        base_color_texture: Some(chunk_material.tex_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        ..default()
    }));
    chunk_material.layers = array_layers;
    chunk_material.loaded = true;
    info!("Loaded chunk material");
}
