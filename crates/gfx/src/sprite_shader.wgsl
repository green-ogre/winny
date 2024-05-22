struct InstanceInput {
    @location(2) world_position: vec4<f32>,
    @location(3) mask: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,    
    @location(0) tex_coords: vec2<f32>,
    @location(1) mask: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    vert: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(vert.position.xyz * 2.0, 1.0) - vec4<f32>(1.0, 1.0, 0.0, 0.0) 
        + vec4<f32>(instance.world_position.xy * 2.0, instance.world_position.z, 0.0);
    // out.clip_position.y = -out.clip_position.y;

    out.tex_coords = vert.tex_coords;
    out.mask = instance.mask;

    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    // var color = vec3<f32>(
    //         clamp((in.mask.x * in.mask.r) + (tex.x * (1.0 - in.mask.r)), 0.0, 1.0), 
    //         clamp((in.mask.y * in.mask.r) + (tex.y * (1.0 - in.mask.r)), 0.0, 1.0),
    //         clamp((in.mask.z * in.mask.r) + (tex.z * (1.0 - in.mask.r)), 0.0, 1.0)
    //     );
    // return vec4<f32>(in.mask.xyz, tex.r);
    return tex;
}

