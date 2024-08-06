struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

struct InstanceInput {
    @location(2) flip_v: f32,
    @location(3) flip_h: f32,
    @location(4) width: u32,
    @location(5) height: u32,
    @location(6) index: u32,
}

struct TransformInput {
    @location(7) m1: vec4<f32>,
    @location(8) m2: vec4<f32>,
    @location(9) m3: vec4<f32>,
    @location(10) m4: vec4<f32>,
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
fn vs_main(
    vert: VertexInput,
    instance: InstanceInput,
    transform: TransformInput,
) -> VertexOutput {
    var out: VertexOutput;
    var transformation_matrix = mat4x4<f32>(
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

    let atlas_position = vec2<f32>(f32(instance.index % instance.width) / f32(instance.width),
        f32(instance.index / instance.width) / f32(instance.height));
    out.uv = vec2<f32>(vert.uv.x / f32(instance.width), vert.uv.y / f32(instance.height)) + atlas_position;
    out.uv.x = (out.uv.x * instance.flip_h) + ((1.0 - out.uv.x) * (1.0 - instance.flip_h));
    out.uv.y = (out.uv.y * instance.flip_v) + ((1.0 - out.uv.y) * (1.0 - instance.flip_v));

    transformation_matrix[0][3] *= 2.0 / camera.viewport_dimensions.x;
    transformation_matrix[1][3] *= -2.0 / camera.viewport_dimensions.y;

    camera_matrix[0][3] *= 2.0 / camera.viewport_dimensions.x;
    camera_matrix[1][3] *= -2.0 / camera.viewport_dimensions.y;

    camera_matrix[0][0] *= camera.viewport_dimensions.x / camera.window_dimensions.x;
    camera_matrix[1][1] *= camera.viewport_dimensions.y / camera.window_dimensions.y;

    out.clip_position = vert.position * camera_matrix;
    out.clip_position *= transformation_matrix;
    out.clip_position.z = 0.0;

    return out;
}











