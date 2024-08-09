struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

struct MaterialUniform {
  modulation: vec4<f32>,
  opacity: f32,
  saturation: f32,
}

@group(1) @binding(0)
var<uniform> mat: MaterialUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return mat.modulation;
}
