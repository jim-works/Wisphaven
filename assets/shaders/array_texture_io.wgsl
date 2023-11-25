#define_import_path recursia::array_texture_io

//from https://github.com/bevyengine/bevy/blob/527d3a5885daa4b43df7054f7787dad47f06135d/crates/bevy_pbr/src/render/mesh.wgsl
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
//my addition tex array layer, ao level
    @location(3) @interpolate(flat) layer: i32,
    @location(4) ao: f32
};

struct VertexOutput {
    //https://github.com/bevyengine/bevy/blob/v0.12.0/crates/bevy_pbr/src/prepass/prepass_io.wgsl
    // this is `clip position` when the struct is used as a vertex stage output 
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) layer: i32,
    @location(4) ao: f32,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>
}