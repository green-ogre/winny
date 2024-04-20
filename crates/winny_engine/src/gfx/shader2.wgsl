struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,    
    @location(0) tex_coords: vec2<f32> ,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    // var out: VertexOutput;
    // var x: f32 = f32((index << 1) & 2);
    // var y: f32 = f32(index & 2);
    // out.tex_coords = vec2<f32>(x, y);
    // out.clip_position = vec4<f32>(out.tex_coords * vec2<f32>(2, -2) + vec2<f32>(-1, 1), 0, 1);

    // return out;

    var out: VertexOutput;
    let x = f32(index & 2) * 2.0 - 1.0;
    let y = f32(index & 1) * 4.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((out.clip_position.x + 1.0) / 4.0, (out.clip_position.y + 1.0) / 4.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.tex_coords, 1.0, 1.0);
}
