struct VertexInput {
    @location(0) position: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct InstanceInput {
  m1: vec4<f32>,
  m2: vec4<f32>,
  m3: vec4<f32>,
  m4: vec4<f32>,
}

@vertex
fn vs_main(
    vert: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let transformation_matrix = mat4x4<f32>(
        instance.m1,
        instance.m2,
        instance.m3,
        instance.m4,
    );

    var out: VertexOutput;
    out.clip_position = vert.position * transformation_matrix;
    out.clip_position.z = 0.0;
    out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

