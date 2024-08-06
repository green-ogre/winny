struct ParticleInstance {
    translation: vec4<f32>,
    velocity: vec4<f32>,
    acceleration: vec4<f32>,
    scale: vec2<f32>,
    creation_time: f32,
    lifetime: f32,
};

@group(0) @binding(0)
var<storage, read_write> particles: array<ParticleInstance>;

struct EmitterUniform {
  initial_velocity: vec4<f32>,
  acceleration: vec4<f32>,
  time_delta: f32,
  time_elapsed: f32,
  width: f32,
  height: f32,
  max_lifetime: f32,
  min_lifetime: f32,
  screen_width: f32,
  screen_height: f32,
}

@group(1) @binding(0)
var<uniform> emitter_uniform: EmitterUniform;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) globalId: vec3u) {
    let index = globalId.x;
    let offset = globalId.x;
    if particles[index].lifetime <= 0.0 {
        var rng = XorRng(u32(emitter_uniform.time_elapsed * f32(offset)));
        let rand = gen_vec3(rng);
        particles[index].translation.x = 2.0 * emitter_uniform.width * rand.x - emitter_uniform.width;
        particles[index].translation.y = 2.0 * emitter_uniform.height * rand.y - emitter_uniform.height;
        particles[index].lifetime = (emitter_uniform.max_lifetime - emitter_uniform.min_lifetime) * rand.z + emitter_uniform.min_lifetime;
        particles[index].creation_time = emitter_uniform.time_elapsed;
        particles[index].velocity = emitter_uniform.initial_velocity;
        particles[index].acceleration = emitter_uniform.acceleration;

        return;
    }

    let a_x = particles[index].acceleration.x;
    let a_y = particles[index].acceleration.y;

    particles[index].translation.x += particles[index].velocity.x / emitter_uniform.screen_width * emitter_uniform.time_delta;
    particles[index].translation.y += particles[index].velocity.y / emitter_uniform.screen_height * emitter_uniform.time_delta;
    particles[index].velocity.x += a_x * emitter_uniform.time_delta;
    particles[index].velocity.y += a_y * emitter_uniform.time_delta;
    particles[index].lifetime -= emitter_uniform.time_delta;
}

struct XorRng {
    state: u32,
}

fn gen(rng: XorRng) -> XorRng {
    var r = rng;
    r.state ^= r.state << 13;
    r.state ^= r.state >> 17;
    r.state ^= r.state << 5;

    return r;
}

fn gen_vec3(rng: XorRng) -> vec3<f32> {
    let x_tmp = gen(rng);
    let y_tmp = gen(rng);
    let z_tmp = gen(rng);
    return vec3<f32>(f32(x_tmp.state) / f32(0xFFFFFFFF), f32(y_tmp.state) / f32(0xFFFFFFFF), f32(z_tmp.state) / f32(0xFFFFFFFF));
}
