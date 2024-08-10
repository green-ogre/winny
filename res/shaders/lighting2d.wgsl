struct VertexInput {
  @location(0) position: vec4<f32>,
}

struct Ambient {
    color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> ambient: Ambient;

struct Light {
    position: vec3<f32>,
    intensity: f32,
    color: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> lights: array<Light, 64>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) light_id: u32,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vert_id: u32, vert: VertexInput) -> VertexOutput {
    // var out: VertexOutput;
    // out.uv = vec2<f32>(f32((vert_id << 1) & 2), f32(vert_id & 2));
    // out.clip_position = vec4<f32>(out.uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    // return out;

    let light = lights[0];
    let id = vert_id % 6;

    // quad[0] = segment[0]
    // quad[1] = segment[1]
    // quad[2] = segment[0] + 100 * (segment[0] - light_position)
    // quad[3] = segment[1] + 100 * (segment[1] - light_position)

    var out: VertexOutput;
    out.clip_position = vert.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // let light = lights[in.light_id];
    // let int = 0.5 - length(light.position.xy - in.uv + vec2(0.5, 0.5));
    // let col = vec4(light.color.xyz, int);
    // return mix(col, ambient.color, ambient.color.a);

    return vec4(0.0, 0.0, 0.0, 1.0);
}
