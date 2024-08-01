struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

struct InstanceInput {
    @builtin(instance_index) index: u32,
    @location(2) flip_v: f32,
    @location(3) flip_h: f32,
}

struct TransformInput {
    @location(4) m1: vec4<f32>,
    @location(5) m2: vec4<f32>,
    @location(6) m3: vec4<f32>,
    @location(7) m4: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct AtlasUniform {
  width: u32,
  height: u32,
  index: u32,
}

@group(0) @binding(0)
var<storage, read> atlas_uniforms: array<AtlasUniform>;

@vertex
fn vs_main(
    vert: VertexInput,
    instance: InstanceInput,
    transform: TransformInput,
) -> VertexOutput {
    var out: VertexOutput;
    var transformation_matrix = mat4x4<f32>(
        transform.m1,
        transform.m2,
        transform.m3,
        transform.m4,
    );

    let atlas = atlas_uniforms[instance.index];
    let atlas_position = vec2<f32>(f32(atlas.index % atlas.width) / f32(atlas.width),
        f32(atlas.index / atlas.width) / f32(atlas.height));
    out.uv = vec2<f32>(vert.uv.x / f32(atlas.width), vert.uv.y / f32(atlas.height)) + atlas_position;
    out.uv.x = (out.uv.x * instance.flip_h) + ((1.0 - out.uv.x) * (1.0 - instance.flip_h));
    out.uv.y = (out.uv.y * instance.flip_v) + ((1.0 - out.uv.y) * (1.0 - instance.flip_v));

    transformation_matrix[0][3] *= 2.0 / 1000.0;
    transformation_matrix[1][3] *= -2.0 / 1000.0;
    transformation_matrix[2][3] *= 1.0 / 1000.0;

    out.clip_position = vert.position * transformation_matrix;
    out.clip_position.z = 0.0;

    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}

