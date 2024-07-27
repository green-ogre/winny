struct ParticleInstance {
    translation: vec4<f32>,
    scale: vec2<f32>,
    rotation: f32,
    lifetime: f32,
};

@group(0) @binding(0)
var<storage, read_write> particles: array<ParticleInstance>;

struct EmitterUniform {
  time_delta: f32,
  time_elapsed: f32,
  width: f32,
  height: f32,
  max_lifetime: f32,
  min_lifetime: f32,
}

@group(1) @binding(0)
var<uniform> emitter_uniform: EmitterUniform;

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;
// @group(2) @binding(1)
// var s_diffuse: sampler;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) globalId: vec3u) {
    let index = globalId.x;
    let offset = globalId.x;
    if particles[index].lifetime <= 0.0 {
        let x_sample = textureLoad(t_diffuse, vec2<u32>((u32(emitter_uniform.time_elapsed * 1000.0) + offset) % 256, 0), 0);
        let y_sample = textureLoad(t_diffuse, vec2<u32>((u32(emitter_uniform.time_elapsed * 480.0) + offset) % 256, 0), 0);
        let l_sample = textureLoad(t_diffuse, vec2<u32>((u32(emitter_uniform.time_elapsed * 242.0) + offset) % 256, 0), 0);
        particles[index].translation.x = 2.0 * emitter_uniform.width * x_sample.x - emitter_uniform.width;
        particles[index].translation.y = 2.0 * emitter_uniform.height * y_sample.x - emitter_uniform.height;
        particles[index].lifetime = (emitter_uniform.max_lifetime - emitter_uniform.min_lifetime) * l_sample.x + emitter_uniform.min_lifetime;

        return;
    }

    particles[index].translation.y -= 0.001;
    particles[index].lifetime -= emitter_uniform.time_delta;
}

