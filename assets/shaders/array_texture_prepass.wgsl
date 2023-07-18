//copied from https://github.com/bevyengine/bevy/blob/main/crates/bevy_pbr/src/render/pbr_prepass.wgsl
//modified to disable alpha cutoff - I couldn't figure out how to pass the texture layer to the fragment input here.
#import bevy_pbr::prepass_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::pbr_types
#ifdef NORMAL_PREPASS
#import bevy_pbr::pbr_functions
#endif // NORMAL_PREPASS

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
#ifdef VERTEX_UVS
    @location(0) uv: vec2<f32>,
#endif // VERTEX_UVS

#ifdef NORMAL_PREPASS
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(2) world_tangent: vec4<f32>,
#endif // VERTEX_TANGENTS
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
    @location(3) world_position: vec4<f32>,
    @location(4) previous_world_position: vec4<f32>,
#endif // MOTION_VECTOR_PREPASS

#ifdef DEPTH_CLAMP_ORTHO
    @location(5) clip_position_unclamped: vec4<f32>,
#endif // DEPTH_CLAMP_ORTHO
};

#ifdef PREPASS_FRAGMENT
struct FragmentOutput {
#ifdef NORMAL_PREPASS
    @location(0) normal: vec4<f32>,
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
    @location(1) motion_vector: vec2<f32>,
#endif // MOTION_VECTOR_PREPASS

#ifdef DEPTH_CLAMP_ORTHO
    @builtin(frag_depth) frag_depth: f32,
#endif // DEPTH_CLAMP_ORTHO
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {

    var out: FragmentOutput;

#ifdef DEPTH_CLAMP_ORTHO
    out.frag_depth = in.clip_position_unclamped.z;
#endif // DEPTH_CLAMP_ORTHO

#ifdef NORMAL_PREPASS
    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if (bevy_pbr::pbr_bindings::material.flags & bevy_pbr::pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        let world_normal = bevy_pbr::pbr_functions::prepare_world_normal(
            in.world_normal,
            (bevy_pbr::pbr_bindings::material.flags & bevy_pbr::pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            in.is_front,
        );

        let normal = bevy_pbr::pbr_functions::apply_normal_mapping(
            bevy_pbr::pbr_bindings::material.flags,
            world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif // STANDARDMATERIAL_NORMAL_MAP
#endif // VERTEX_TANGENTS
#ifdef VERTEX_UVS
            in.uv,
#endif // VERTEX_UVS
            bevy_pbr::prepass_bindings::view.mip_bias,
        );

        out.normal = vec4(normal * 0.5 + vec3(0.5), 1.0);
    } else {
        out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
    }
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
    let clip_position_t = bevy_pbr::prepass_bindings::view.unjittered_view_proj * in.world_position;
    let clip_position = clip_position_t.xy / clip_position_t.w;
    let previous_clip_position_t = bevy_pbr::prepass_bindings::previous_view_proj * in.previous_world_position;
    let previous_clip_position = previous_clip_position_t.xy / previous_clip_position_t.w;
    // These motion vectors are used as offsets to UV positions and are stored
    // in the range -1,1 to allow offsetting from the one corner to the
    // diagonally-opposite corner in UV coordinates, in either direction.
    // A difference between diagonally-opposite corners of clip space is in the
    // range -2,2, so this needs to be scaled by 0.5. And the V direction goes
    // down where clip space y goes up, so y needs to be flipped.
    out.motion_vector = (clip_position - previous_clip_position) * vec2(0.5, -0.5);
#endif // MOTION_VECTOR_PREPASS

    return out;
}
#else
@fragment
fn fragment(in: FragmentInput) {
}
#endif // PREPASS_FRAGMENT