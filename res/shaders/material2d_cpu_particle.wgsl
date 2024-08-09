struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

struct MaterialUniform {
  modulation: vec4<f32>,
  opacity: f32,
  saturation: f32,
}

@group(1) @binding(2)
var<uniform> mat: MaterialUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(texture, texture_sampler, in.uv);
}
