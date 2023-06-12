// #import bevy_pbr::mesh_view_bindings
// #import bevy_pbr::mesh_bindings

// //#import bevy_pbr::pbr_types
// #import bevy_pbr::pbr_bindings
// #import bevy_pbr::utils
// #import bevy_pbr::clustered_forward
// #import bevy_pbr::lighting
// #import bevy_pbr::shadows
// #import bevy_pbr::fog
// #import bevy_pbr::pbr_functions
// #import bevy_pbr::pbr_ambient

#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::mesh_bindings

#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::pbr_ambient
#import bevy_pbr::shadows
#import bevy_pbr::fog
#import bevy_pbr::pbr_functions

#import bevy_pbr::prepass_utils
#import bevy_pbr::mesh_functions

@group(1) @binding(0)
var my_array_texture: texture_2d_array<f32>;
@group(1) @binding(1)
var my_array_texture_sampler: sampler;

//from https://github.com/bevyengine/bevy/blob/527d3a5885daa4b43df7054f7787dad47f06135d/crates/bevy_pbr/src/render/mesh.wgsl
struct Vertex {
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif
//my addition (it doesn't like u32 when sampling the texture for some reason)
    @location(7) layer: i32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(5) layer: i32,
    //uses locations 0-4
    #import bevy_pbr::mesh_vertex_output
    
};

//from https://github.com/bevyengine/bevy/blob/527d3a5885daa4b43df7054f7787dad47f06135d/crates/bevy_pbr/src/render/mesh.wgsl
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef SKINNED
    var model = skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    var model = mesh.model;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skin_normals(model, vertex.normal);
#else
    out.world_normal = mesh_normal_local_to_world(vertex.normal);
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.clip_position = mesh_position_world_to_clip(out.world_position);
#endif

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_tangent_local_to_world(model, vertex.tangent);
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
//my addition
    out.layer = vertex.layer;
    return out;
}


struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    //uses locations 0-4
    @location(5) layer: i32,
    #import bevy_pbr::mesh_vertex_output
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
    // the material members
    //TODO: update according to https://github.com/bevyengine/bevy/blob/527d3a5885daa4b43df7054f7787dad47f06135d/crates/bevy_pbr/src/render/pbr.wgsl
    var pbr_input: PbrInput = pbr_input_new();

    //https://www.w3.org/TR/WGSL/#texturesample
    pbr_input.material.base_color = textureSample(my_array_texture, my_array_texture_sampler, in.uv, in.layer);
#ifdef VERTEX_COLORS
    pbr_input.material.base_color = pbr_input.material.base_color * in.color;
#endif

    pbr_input.frag_coord = in.frag_coord;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = prepare_world_normal(
        in.world_normal,
        (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
        in.is_front,
    );

    pbr_input.is_orthographic = view.projection[3].w == 1.0;

    pbr_input.N = apply_normal_mapping(
        pbr_input.material.flags,
        pbr_input.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        in.world_tangent,
#endif
#endif
        in.uv,
    );
    pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);

    return tone_mapping(pbr(pbr_input));
}