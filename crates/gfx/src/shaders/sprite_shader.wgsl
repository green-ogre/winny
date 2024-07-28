struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

struct InstanceInput {
    @builtin(instance_index) index: u32,
    @location(2) mask: vec4<f32>,
    @location(3) flip_v: f32,
    @location(4) flip_h: f32,
}

struct TransformInput {
    @location(5) m1: vec4<f32>,
    @location(6) m2: vec4<f32>,
    @location(7) m3: vec4<f32>,
    @location(8) m4: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) flip_h: f32,
    @location(2) flip_v: f32,
    @location(3) mask: vec4<f32>,
}

struct AtlasUniform {
  width: u32,
  height: u32,
  index: u32,
}

@group(1) @binding(0)
var<storage, read> atlas_uniforms: array<AtlasUniform>;

@vertex
fn vs_main(
    vert: VertexInput,
    instance: InstanceInput,
    transform: TransformInput,
) -> VertexOutput {
    var out: VertexOutput;
    let transformation_matrix = mat4x4<f32>(
        transform.m1,
        transform.m2,
        transform.m3,
        transform.m4,
    );

    let atlas = atlas_uniforms[instance.index];
    let atlas_position = vec2<f32>(f32(atlas.index % atlas.width) / f32(atlas.width),
        f32(atlas.index / atlas.width) / f32(atlas.height));
    out.uv = vec2<f32>(vert.uv.x / f32(atlas.width), vert.uv.y / f32(atlas.height)) + atlas_position;

    out.clip_position = vert.position * transformation_matrix;
    out.clip_position.z = 0.0;
    out.mask = instance.mask;
    out.flip_h = instance.flip_h;
    out.flip_v = instance.flip_v;

    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x = (in.uv.x * in.flip_h) + ((1.0 - in.uv.x) * (1.0 - in.flip_h));
    let y = (in.uv.y * in.flip_v) + ((1.0 - in.uv.y) * (1.0 - in.flip_v));
    var tex = textureSample(t_diffuse, s_diffuse, vec2<f32>(x, y));
    let alpha_mask = step(0.001, tex.a);
    let output_color = mix(tex.rgb, in.mask.rgb, in.mask.a * alpha_mask);
    return vec4<f32>(output_color, tex.a);
}

