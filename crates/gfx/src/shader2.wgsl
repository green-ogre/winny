struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,    
    // @location(0) color: vec3<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) num_friends: u32,
    @location(3) rotation: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

// @vertex
// fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
//     var out: VertexOutput;
// 
//     let x = f32(index & 2) * 2.0 - 1.0;
//     let y = f32(index & 1) * 4.0 - 1.0;
//     out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
// 
//     let tex_x = (out.clip_position.x + 1.0) / 2.0;
//     let tex_y = 1.0 - ((out.clip_position.y + 1.0) / 2.0);
//     out.tex_coords = vec2<f32>(tex_x, tex_y);
// 
//     return out;
// }

// @vertex
// fn vs_main(boid_vert: VertexInput) -> VertexOutput {
//     var out: VertexOutput;
//     out.clip_position = vec4<f32>(boid_vert.position, 1.0);
//     out.tex_coords = vec2<f32>(boid_vert.tex_coord.x, boid_vert.tex_coord.y);
// 
//     return out;
// }

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@group(2) @binding(0)
var t_color: texture_2d<f32>;
@group(2)@binding(1)
var s_color: sampler;

@group(3) @binding(0)
var rot_t_color: texture_2d<f32>;
@group(3)@binding(1)
var rot_s_color: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Color by number of friends
    // var tex_alpha = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    // var boid_color = textureSample(t_color, s_color, vec2<f32>(0.0, f32(in.num_friends) / 200.0));
    // return vec4<f32>(boid_color.xyz, tex_alpha.r);

    // Color by rotation
    var tex_alpha = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    var boid_color = textureSample(rot_t_color, rot_s_color, vec2<f32>(0.0, in.rotation));
    return vec4<f32>(boid_color.xyz, tex_alpha.r);
}




struct InstanceInput {
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
    @location(6) color: vec4<f32>,
    @location(7) num_friends: u32,
    @location(8) rotation: f32,
}

@vertex
fn vs_main(
    vert: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let instance_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_position = instance_matrix * vec4<f32>(vert.position, 1.0);

    var out: VertexOutput;
    out.clip_position = (camera.view_proj * world_position) - vec4<f32>(1.0, 1.0, 0.0, 0.0);
    out.tex_coords = vert.tex_coords;
    out.color = instance.color;
    out.num_friends = instance.num_friends;
    out.rotation = instance.rotation;

    return out;
}


struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;
