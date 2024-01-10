#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

#import bevy_pbr::mesh_functions

@group(1) @binding(100) var array_texture: texture_2d_array<f32>;
@group(1) @binding(101) var texture_sampler: sampler;

struct ChunkVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) layer: i32,
    @location(4) ao: f32,
}

struct ChunkVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) layer: i32,
    @location(4) ao: f32,
}

@vertex
fn vertex(vertex: ChunkVertex) -> ChunkVertexOutput {
    var out: ChunkVertexOutput;
    var model = mesh_functions::get_model_matrix(vertex.instance_index);
    out.clip_position = mesh_functions::mesh_position_local_to_clip(
        model,
        vec4<f32>(vertex.position, 1.0),
    );
    out.world_position = mesh_functions::mesh_position_local_to_world(
        model,
        vec4<f32>(vertex.position, 1.0),
    );
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index,
    );
    out.uv = vertex.uv;
    out.layer = vertex.layer;
    out.ao = vertex.ao;
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    in: ChunkVertexOutput,
) -> FragmentOutput {

    var pbr_vertex: VertexOutput;
    pbr_vertex.position = in.clip_position;
    pbr_vertex.world_position = in.world_position;
    pbr_vertex.world_normal = in.world_normal;
    pbr_vertex.uv = in.uv;

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(pbr_vertex, is_front);

    // overwrite color with sample from texture array
    pbr_input.material.base_color = textureSample(array_texture, texture_sampler, in.uv, in.layer);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.xyz * in.ao, pbr_input.material.base_color.w);
#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif
    
    return out;
}