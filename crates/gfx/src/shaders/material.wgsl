@group(0) @binding(0)
var<uniform> material: MaterialUniform;

struct MaterialUniform {
  modulation: vec4<f32>,
  opacity: f32,
  saturation: f32,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vert_id: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vert_id) & 1);
    let y = f32(i32(vert_id) >> 1);
    out.clip_position = vec4<f32>(
        x * 4.0 - 1.0,
        y * 4.0 - 1.0,
        0.0,
        1.0
    );
    out.uv = vec2<f32>(x * 2.0, 1.0 - y * 2.0);
    return out;
}

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex = textureSample(texture, texture_sampler, in.uv);
    // let alpha_mask = step(0.001, tex.a);
    // let output_color = mix(tex.rgb, in.mask.rgb, in.mask.a * alpha_mask);
    // return vec4<f32>(output_color, tex.a * in.opacity);
    return tex;
}
