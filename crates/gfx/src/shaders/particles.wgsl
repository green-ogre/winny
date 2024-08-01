struct EmitterUniform {
    m1: vec4<f32>,
    m2: vec4<f32>,
    m3: vec4<f32>,
    m4: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> emitter: EmitterUniform;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

struct InstanceInput {
    @location(2) alive_index: u32,
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
fn vs_main(vert: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    var particle_transformation = mat4x4<f32>(
        emitter.m1,
        emitter.m2,
        emitter.m3,
        emitter.m4,
    );

    particle_transformation[0][3] *= 2.0 / 1000.0;
    particle_transformation[1][3] *= -2.0 / 1000.0;
    particle_transformation[2][3] *= 1.0 / 1000.0;

    let particle = particle_storage[instance.alive_index];
    out.clip_position = vert.position * particle_transformation;
    out.clip_position += vec4<f32>(particle.translation.xy, 0.0, 0.0);
    out.clip_position.z = 0.0;
    out.uv = vert.uv;
    return out;
}
