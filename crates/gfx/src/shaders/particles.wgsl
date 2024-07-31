struct EmitterUniform {
    m1: vec4<f32>,
    m2: vec4<f32>,
    m3: vec4<f32>,
    m4: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> emitter: EmitterUniform;

struct InstanceInput {
    @location(0) alive_index: u32,
}

@group(1) @binding(0)
var<storage, read> particle_storage: array<ParticleInstance>;

struct ParticleInstance {
  translation: vec4<f32>,
  velocity: vec4<f32>,
  acceleration: vec4<f32>,
  scale: vec2<f32>,
  creation_time: f32,
  lifetime: f32,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vert_id: u32, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    let x = f32(i32(vert_id) & 1);
    let y = f32(i32(vert_id) >> 1);
    out.clip_position = vec4<f32>(
        x * 4.0 - 1.0,
        y * 4.0 - 1.0,
        0.0,
        1.0
    );

    let particle_transformation = mat4x4<f32>(
        emitter.m1,
        emitter.m2,
        emitter.m3,
        emitter.m4,
    );

    let particle = particle_storage[instance.alive_index];
    out.clip_position *= particle_transformation;
    out.clip_position += vec4<f32>(particle.translation.xy, 0.0, 0.0);
    out.clip_position.z = 0.0;
    out.uv = vec2<f32>(x * 2.0, 1.0 - y * 2.0);
    return out;
}

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let in_bounds = step(vec2<f32>(0.0), in.uv) * step(in.uv, vec2<f32>(1.0));
    let factor = in_bounds.x * in_bounds.y;
    let texel = textureSample(texture, texture_sampler, in.uv);
    return texel * factor;
}
