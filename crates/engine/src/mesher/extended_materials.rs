use bevy::{prelude::*, render::render_resource::{AsBindGroup, ShaderRef}, pbr::MaterialExtension};

use super::materials::{ATTRIBUTE_TEXLAYER, ATTRIBUTE_AO};

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct TextureArrayExtension {
    #[texture(100, dimension = "2d_array")]
    #[sampler(101)]
    #[dependency]
    pub base_color_texture: Option<Handle<Image>>,
}

impl MaterialExtension for TextureArrayExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/texture_array.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/texture_array.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/texture_array.wgsl".into()
    }

    fn specialize(
            _pipeline: &bevy::pbr::MaterialExtensionPipeline,
            descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
            layout: &bevy::render::mesh::MeshVertexBufferLayout,
            _key: bevy::pbr::MaterialExtensionKey<Self>,
        ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
            let vertex_layout = layout.get_layout(&[
                //standard bevy pbr stuff (check assets/shaders/array_texture.wgsl Vertex struct)
                Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
                Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
                Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
                //my addition
                ATTRIBUTE_TEXLAYER.at_shader_location(3),
                ATTRIBUTE_AO.at_shader_location(4),
            ])?;
            descriptor.vertex.buffers = vec![vertex_layout];
            Ok(())
    }
}

//encodes texture layer as a rgba32 color
//jank that allows us to re use the chunk meshing code for generating item meshes
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ColorArrayExtension {

}

impl MaterialExtension for ColorArrayExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/color_array.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/color_array.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/color_array.wgsl".into()
    }

    fn specialize(
            _pipeline: &bevy::pbr::MaterialExtensionPipeline,
            descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
            layout: &bevy::render::mesh::MeshVertexBufferLayout,
            _key: bevy::pbr::MaterialExtensionKey<Self>,
        ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
            let vertex_layout = layout.get_layout(&[
                //standard bevy pbr stuff (check assets/shaders/array_texture.wgsl Vertex struct)
                Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
                Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
                Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
                //texlayer is rgba32 color, uses little endian with r as least significant
                ATTRIBUTE_TEXLAYER.at_shader_location(3),
                ATTRIBUTE_AO.at_shader_location(4),
            ])?;
            descriptor.vertex.buffers = vec![vertex_layout];
            Ok(())
    }
}