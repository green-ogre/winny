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
        let rand = random(emitter_uniform.time_elapsed + f32(offset));
        particles[index].translation.x = 2.0 * emitter_uniform.width * rand.x - emitter_uniform.width;
        particles[index].translation.y = 2.0 * emitter_uniform.height * rand.y - emitter_uniform.height;
        particles[index].lifetime = (emitter_uniform.max_lifetime - emitter_uniform.min_lifetime) * rand.y + emitter_uniform.min_lifetime;
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

fn random(seed: f32) -> vec3<f32> {
    let a = 12.9898;
    let b = 78.233;
    let c = 43758.5453;

    let x = fract(sin(dot(vec2<f32>(seed, seed), vec2<f32>(a, b))) * c);
    let y = fract(sin(dot(vec2<f32>(seed + x, x), vec2<f32>(b, a))) * c);
    let z = fract(sin(dot(vec2<f32>(y, x + y), vec2<f32>(a, b))) * c);

    return vec3<f32>(x, y, z);
}

// TODO: integer hashing

// /// A simple XOR-based RNG.
// pub struct XorRng {
//     state: u32,
// }
// 
// impl XorRng {
//     /// Generate a new random number, updating the RNG's internal state.
//     pub fn generate(&mut self) -> u32 {
//         self.state ^= self.state << 13;
//         self.state ^= self.state >> 17;
//         self.state ^= self.state << 5;
// 
//         self.state
//     }
// }
