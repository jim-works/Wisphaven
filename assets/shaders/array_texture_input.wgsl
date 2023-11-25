#define_import_path recursia::array_texture_input

#import bevy_pbr::{
    pbr_functions,
    pbr_bindings,
    pbr_types,
    prepass_utils,
    mesh_bindings::mesh,
    mesh_view_bindings::view,
    parallax_mapping::parallaxed_uv,
}

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::mesh_view_bindings::screen_space_ambient_occlusion_texture
#import bevy_pbr::gtao_utils::gtao_multibounce
#endif

#import recursia::array_texture_io::VertexOutput

//overriding to use array textures: https://github.com/bevyengine/bevy/blob/main/crates/bevy_pbr/src/render/pbr_bindings.wgsl
//binding 0 is starndard material
@group(1) @binding(1)
var array_color_texture: texture_2d_array<f32>;
@group(1) @binding(2)
var array_color_texture_sampler: sampler;
//disabled - see fragment function
@group(1) @binding(3)
var array_emissive_texture: texture_2d_array<f32>;
@group(1) @binding(4)
var array_emissive_texture_sampler: sampler;
//disabled - see fragment function
@group(1) @binding(5)
var array_metallic_roughness_texture: texture_2d_array<f32>;
@group(1) @binding(6)
var array_metallic_roughness_texture_sampler: sampler;
//disabled - see fragment function
@group(1) @binding(7)
var array_occlusion_texture: texture_2d_array<f32>;
@group(1) @binding(8)
var array_occlusion_texture_sampler: sampler;
@group(1) @binding(9)
var array_normal_map_texture: texture_2d_array<f32>;
@group(1) @binding(10)
var array_normal_map_texture_sampler: sampler;
@group(1) @binding(11)
var array_depth_map_texture: texture_2d_array<f32>;
@group(1) @binding(12)
var array_depth_map_texture_sampler: sampler;
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
@group(1) @binding(13) var array_specular_transmission_texture: texture_2d_array<f32>;
@group(1) @binding(14) var array_specular_transmission_sampler: sampler;
@group(1) @binding(15) var array_thickness_texture: texture_2d_array<f32>;
@group(1) @binding(16) var array_thickness_sampler: sampler;
@group(1) @binding(17) var array_diffuse_transmission_texture: texture_2d_array<f32>;
@group(1) @binding(18) var array_diffuse_transmission_sampler: sampler;
#endif

// prepare a basic PbrInput from the vertex stage output, mesh binding and view binding
fn pbr_input_from_vertex_output(
    in: VertexOutput,
    is_front: bool,
    double_sided: bool,
) -> pbr_types::PbrInput {
    var pbr_input: pbr_types::PbrInput = pbr_types::pbr_input_new();

    pbr_input.flags = mesh[0].flags;
    pbr_input.is_orthographic = view.projection[3].w == 1.0;
    pbr_input.V = pbr_functions::calculate_view(in.world_position, pbr_input.is_orthographic);
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;

#ifdef VERTEX_COLORS
    pbr_input.material.base_color = in.color;
#endif

    pbr_input.world_normal = pbr_functions::prepare_world_normal(
        in.world_normal,
        double_sided,
        is_front,
    );

#ifdef LOAD_PREPASS_NORMALS
    pbr_input.N = prepass_utils::prepass_normal(in.position, 0u);
#else
    pbr_input.N = normalize(pbr_input.world_normal);
#endif

    return pbr_input;
}

// Prepare a full PbrInput by sampling all textures to resolve
// the material members
fn pbr_input_from_array_texture_material(
    in: VertexOutput,
    is_front: bool,
) -> pbr_types::PbrInput {
    let double_sided = (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    var pbr_input: pbr_types::PbrInput = pbr_input_from_vertex_output(in, is_front, double_sided);
    pbr_input.material.flags = pbr_bindings::material.flags;
    pbr_input.material.base_color *= pbr_bindings::material.base_color;
    pbr_input.material.deferred_lighting_pass_id = pbr_bindings::material.deferred_lighting_pass_id;

    var uv = in.uv;
    var layer = in.layer;

    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        pbr_input.material.base_color *= textureSample(array_color_texture, array_color_texture_sampler, uv, layer);
    }

    pbr_input.material.flags = pbr_bindings::material.flags;

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        pbr_input.material.reflectance = pbr_bindings::material.reflectance;
        pbr_input.material.ior = pbr_bindings::material.ior;
        pbr_input.material.attenuation_color = pbr_bindings::material.attenuation_color;
        pbr_input.material.attenuation_distance = pbr_bindings::material.attenuation_distance;
        pbr_input.material.alpha_cutoff = pbr_bindings::material.alpha_cutoff;

        // emissive
        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = pbr_bindings::material.emissive;
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSample(array_emissive_texture, array_emissive_texture_sampler, uv, layer).rgb, 1.0);
        }
        pbr_input.material.emissive = emissive;

        // metallic and perceptual roughness
        var metallic: f32 = pbr_bindings::material.metallic;
        var perceptual_roughness: f32 = pbr_bindings::material.perceptual_roughness;
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(array_metallic_roughness_texture, array_metallic_roughness_texture_sampler, uv, layer);
            // Sampling from GLTF standard channels for now
            metallic *= metallic_roughness.b;
            perceptual_roughness *= metallic_roughness.g;
        }
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        var specular_transmission: f32 = pbr_bindings::material.specular_transmission;
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_SPECULAR_TRANSMISSION_TEXTURE_BIT) != 0u) {
            specular_transmission *= textureSample(array_specular_transmission_texture, array_specular_transmission_sampler, uv, layer).r;
        }
#endif
        pbr_input.material.specular_transmission = specular_transmission;

        var thickness: f32 = pbr_bindings::material.thickness;
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_THICKNESS_TEXTURE_BIT) != 0u) {
            thickness *= textureSample(array_thickness_texture, array_thickness_sampler, uv, layer).g;
        }
#endif
        pbr_input.material.thickness = thickness;

        var diffuse_transmission = pbr_bindings::material.diffuse_transmission;
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DIFFUSE_TRANSMISSION_TEXTURE_BIT) != 0u) {
            diffuse_transmission *= textureSample(array_diffuse_transmission_texture, array_diffuse_transmission_sampler, uv, layer).a;
        }
#endif
        pbr_input.material.diffuse_transmission = diffuse_transmission;

        // occlusion
        // TODO: Split into diffuse/specular occlusion?
        var occlusion: vec3<f32> = vec3(1.0);
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = vec3(textureSample(array_occlusion_texture, array_occlusion_texture_sampler, uv, layer).r);
        }
#endif
#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ssao_multibounce = gtao_multibounce(ssao, pbr_input.material.base_color.rgb);
        occlusion = min(occlusion, ssao_multibounce);
#endif
        pbr_input.occlusion = occlusion;

        // N (normal vector)
#ifndef LOAD_PREPASS_NORMALS
        pbr_input.N = pbr_functions::apply_normal_mapping(
            pbr_bindings::material.flags,
            pbr_input.world_normal,
            double_sided,
            is_front,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif
#endif
#ifdef VERTEX_UVS
            uv,
#endif
            view.mip_bias,
        );
#endif
    }

    return pbr_input;
}