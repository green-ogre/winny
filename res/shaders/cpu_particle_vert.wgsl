struct ParticleTransform {
    @location(2) m1: vec4<f32>,
    @location(3) m2: vec4<f32>,
    @location(4) m3: vec4<f32>,
    @location(5) m4: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

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

struct CameraUniform {
    m1: vec4<f32>,
    m2: vec4<f32>,
    m3: vec4<f32>,
    m4: vec4<f32>,
    viewport_dimensions: vec2<f32>,
    window_dimensions: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(vert: VertexInput, transform: ParticleTransform) -> VertexOutput {
    var out: VertexOutput;
    var particle_transformation = mat4x4<f32>(
        transform.m1,
        transform.m2,
        transform.m3,
        transform.m4,
    );

    var camera_matrix = mat4x4<f32>(
        camera.m1,
        camera.m2,
        camera.m3,
        camera.m4,
    );

    let x_scale = 2.0 / camera.window_dimensions.x;
    let y_scale = -2.0 / camera.window_dimensions.y;

    // particle_transformation[0][3] *= x_scale;
    // particle_transformation[1][3] *= y_scale;
    camera_matrix[0][3] *= x_scale;
    camera_matrix[1][3] *= y_scale;

    camera_matrix[0][0] *= camera.viewport_dimensions.x / camera.window_dimensions.x;
    camera_matrix[1][1] *= camera.viewport_dimensions.y / camera.window_dimensions.y;

    out.clip_position = vert.position * particle_transformation;
    out.clip_position *= camera_matrix;
    out.clip_position.z = 0.0;
    out.uv = vert.uv;
    return out;
}
