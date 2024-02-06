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

//exists to turn chunk format into colored pixel format, reusing the same structure
//  so that we can reuse the meshing code (I'm lazy)

struct ColorVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: i32,
    @location(4) ao: f32,
}

struct ColorVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) color: i32,
    @location(4) ao: f32,
}

@vertex
fn vertex(vertex: ColorVertex) -> ColorVertexOutput {
    var out: ColorVertexOutput;
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
    out.color = vertex.color;
    out.ao = vertex.ao;
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    in: ColorVertexOutput,
) -> FragmentOutput {

    var pbr_vertex: VertexOutput;
    pbr_vertex.position = in.clip_position;
    pbr_vertex.world_position = in.world_position;
    pbr_vertex.world_normal = in.world_normal;
    pbr_vertex.uv = in.uv;

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(pbr_vertex, is_front);

    // extract color from vertex rgba32 format
    let rgba = in.color;
    let a = f32((rgba >> 24u) & 0xFF) / 255.0f;
    let b = f32((rgba >> 16u) & 0xFF) / 255.0f;
    let g = f32((rgba >> 8u) & 0xFF) / 255.0f;
    let r = f32(rgba & 0xFF) / 255.0f;
    pbr_input.material.base_color = vec4<f32>(r,g,b,a);

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