struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,    
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;

    let x = f32(index & 2) * 2.0 - 1.0;
    let y = f32(index & 1) * 4.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);

    let tex_x = (out.clip_position.x + 1.0) / 2.0;
    let tex_y = 1.0 - ((out.clip_position.y + 1.0) / 2.0);
    out.tex_coords = vec2<f32>(tex_x, tex_y);

    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
