struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vert_id: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(f32((vert_id << 1) & 2), f32(vert_id & 2));
    out.clip_position = vec4<f32>(out.uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}
