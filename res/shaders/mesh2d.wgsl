struct VertexInput {
  @location(0) position: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

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

struct TransformInput {
    @location(1) m1: vec4<f32>,
    @location(2) m2: vec4<f32>,
    @location(3) m3: vec4<f32>,
    @location(4) m4: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vert_id: u32, vert: VertexInput, transform: TransformInput) -> VertexOutput {
    var camera_matrix = mat4x4<f32>(
        camera.m1,
        camera.m2,
        camera.m3,
        camera.m4,
    );

    var transformation_matrix = mat4x4<f32>(
        transform.m1,
        transform.m2,
        transform.m3,
        transform.m4,
    );

    let x_scale = 2.0 / camera.viewport_dimensions.x;
    let y_scale = -2.0 / camera.viewport_dimensions.y;

    transformation_matrix[0][3] *= x_scale;
    transformation_matrix[1][3] *= y_scale;
    camera_matrix[0][3] *= x_scale;
    camera_matrix[1][3] *= y_scale;

    camera_matrix[0][0] *= camera.viewport_dimensions.x / camera.window_dimensions.x;
    camera_matrix[1][1] *= camera.viewport_dimensions.y / camera.window_dimensions.y;

    var out: VertexOutput;
    out.clip_position = vert.position;
    out.clip_position.x *= 2.0 / camera.window_dimensions.x;
    out.clip_position.y *= 2.0 / camera.window_dimensions.y;
    out.clip_position = out.clip_position * transformation_matrix * camera_matrix;
    out.clip_position.z = 0.0;
    // out.uv = vec2<f32>(f32((vert_id << 1) & 2), f32(vert_id & 2));
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 0.0, 0.0, 1.0);
}
