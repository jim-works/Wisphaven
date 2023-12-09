#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_functions
#import bevy_render::instance_index::get_instance_index

#import bevy_pbr::{
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}

#import wisphaven::{
    array_texture_input::pbr_input_from_array_texture_material,
    array_texture_io::{Vertex, VertexOutput, FragmentOutput}
}



//@group(1) @binding(50)
//const ao_curve = array(1.0, 0.9, 0.6, 0.3);

//from https://github.com/bevyengine/bevy/blob/527d3a5885daa4b43df7054f7787dad47f06135d/crates/bevy_pbr/src/render/mesh.wgsl
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var model = mesh_functions::get_model_matrix(vertex.instance_index);

    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, get_instance_index(vertex.instance_index));

    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);

    out.uv = vertex.uv;

//my addition
    out.layer = vertex.layer;
    out.ao = vertex.ao;
    return out;
}

//copied from bevy's pbr shader https://github.com/bevyengine/bevy/blob/main/crates/bevy_pbr/src/render/pbr.wgsl
//I changed all textureSample and textureSampleBias calls to use `in.layer`
//  -   and multiply the output color by `in.ao`
//  -   disabled emissive, occlusion, and metiallic_roughness textures. the shader will crash even with the `if` checking the flags
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_array_texture_material(in, is_front);

    // alpha discard
    // pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    //my addition - ao
    out.color = vec4<f32>(out.color.xyz * in.ao, out.color.w);
    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}