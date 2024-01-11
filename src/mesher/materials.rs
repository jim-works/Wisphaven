use bevy::{
    pbr::*,
    prelude::*,
    render::{
        render_resource::{
            Extent3d, TextureFormat,
            TextureViewDescriptor, TextureViewDimension, VertexFormat,
        },
        texture::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor, TextureFormatPixelInfo}, mesh::MeshVertexAttribute,
    },
};

use crate::world::settings::Settings;

use super::{TerrainTexture, extended_materials::TextureArrayExtension};

pub const PIXELS_PER_BLOCK: u32 = 16;
//random high id to not conflict
//would make more sense to be u32, but the texture sampler in the shader doesn't like u32 for some reason
pub const ATTRIBUTE_TEXLAYER: MeshVertexAttribute =
    MeshVertexAttribute::new("TexLayer", 970540917, VertexFormat::Sint32);
pub const ATTRIBUTE_AO: MeshVertexAttribute =
    MeshVertexAttribute::new("AOLevel", 970540918, VertexFormat::Float32);
    
#[derive(Resource)]
pub struct ChunkMaterial {
    tex_handle: Option<Handle<Image>>,
    pub opaque_material: Option<Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>>,
    pub transparent_material: Option<Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>>,
    pub loaded: bool,
}

pub fn init(mut commands: Commands) {
    commands.insert_resource(ChunkMaterial {
        tex_handle: None,
        opaque_material: None,
        transparent_material: None,
        loaded: false,
    });
}

fn create_chunk_texture(
    settings: &Settings,
    images: &mut Assets<Image>,
    textures: &TerrainTexture,
) -> Handle<Image> {
    let format = TextureFormat::Rgba8UnormSrgb;
    info!("creating chunk texture with {} images", textures.0.len());
    //copy texture in order into texture array
    let mut image_data = Vec::with_capacity(
        format.pixel_size()
            * settings.block_tex_size.x as usize
            * settings.block_tex_size.y as usize
            * textures.0.len(),
    );
    for handle in textures.0.iter() {
        let image = images.get(handle).unwrap();
        assert_eq!(image.size().x, settings.block_tex_size.x);
        assert_eq!(image.size().y, settings.block_tex_size.y);
        if format != image.texture_descriptor.format {
            //automatically convert format if needed
            warn!(
                "Loading a texture of format '{:?}' when it should have format '{:?}'",
                image.texture_descriptor.format, format
            );
            let converted = image.convert(format).unwrap();
            image_data.extend(converted.data);
        } else {
            image_data.extend(image.data.iter());
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: settings.block_tex_size.x,
            height: settings.block_tex_size.y,
            depth_or_array_layers: textures.0.len() as u32,
        },
        bevy::render::render_resource::TextureDimension::D2,
        image_data,
        format,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    //set filtering for clean pixel art, repeat textures for greedy meshing
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::nearest()
    });
    images.add(image)
}

pub fn create_chunk_material(
    mut chunk_material: ResMut<ChunkMaterial>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>>,
    mut images: ResMut<Assets<Image>>,
    mut block_textures: ResMut<TerrainTexture>,
    settings: Res<Settings>,
) {
    //skip if already loaded
    if chunk_material.loaded {
        return;
    }
    chunk_material.tex_handle = Some(create_chunk_texture(
        &settings,
        images.as_mut(),
        &block_textures,
    ));
    block_textures.0.clear();

    let base = StandardMaterial {
        alpha_mode: AlphaMode::Opaque,
        perceptual_roughness: 1.0,
        reflectance: 0.25,
        ..default()
    };

    chunk_material.opaque_material = Some(materials.add(ExtendedMaterial {
        base: base.clone(),
        extension: TextureArrayExtension {
            base_color_texture: Some(chunk_material.tex_handle.clone().unwrap()),
        }
    }));
    chunk_material.transparent_material = Some(materials.add(ExtendedMaterial {
        base: StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            ..base.clone()
        },
        extension: TextureArrayExtension {
            base_color_texture: Some(chunk_material.tex_handle.clone().unwrap()),
        }
    }));
    chunk_material.loaded = true;
    info!("Loaded chunk material");
}
