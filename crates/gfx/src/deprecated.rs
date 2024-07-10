
// Vertex shader

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(2) @binding(0)
var<uniform> light: Light;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tangent_position: vec3<f32>,
    @location(2) tangent_light_position: vec3<f32>,
    @location(3) tangent_view_position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    // Construct the tangent matrix
    let world_normal = normalize(normal_matrix * model.normal);
    let world_tangent = normalize(normal_matrix * model.tangent);
    let world_bitangent = normalize(normal_matrix * model.bitangent);
    let tangent_matrix = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal,
    ));

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.tex_coords = model.tex_coords;
    out.tangent_position = tangent_matrix * world_position.xyz;
    out.tangent_view_position = tangent_matrix * camera.view_pos.xyz;
    out.tangent_light_position = tangent_matrix * light.position;
    return out;
}
//
// // Fragment shader
//
// @group(0) @binding(0)
// var t_diffuse: texture_2d<f32>;
// @group(0)@binding(1)
// var s_diffuse: sampler;
// @group(0)@binding(2)
// var t_normal: texture_2d<f32>;
// @group(0) @binding(3)
// var s_normal: sampler;
//
// @fragment
// fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//     let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
//     let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
//
//     // We don't need (or want) much ambient light, so 0.1 is fine
//     let ambient_strength = 0.1;
//     let ambient_color = light.color * ambient_strength;
//
//     // Create the lighting vectors
//     let tangent_normal = object_normal.xyz * 2.0 - 1.0;
//     let light_dir = normalize(in.tangent_light_position - in.tangent_position);
//     let view_dir = normalize(in.tangent_view_position - in.tangent_position);
//     let half_dir = normalize(view_dir + light_dir);
//
//     let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
//     let diffuse_color = light.color * diffuse_strength;
//
//     let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
//     let specular_color = specular_strength * light.color;
//
//     let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;
//
//     return vec4<f32>(result, object_color.a);
// }
//
//
// use std::f32::consts::FRAC_PI_2;
//
// use cgmath::perspective;
// use cgmath::prelude::*;
// use cgmath::Matrix4;
// use cgmath::Point3;
// use cgmath::Rad;
// use cgmath::SquareMatrix;
// use cgmath::Vector3;
// use ecs::WinnyResource;
// use plugins::Plugin;
//
// use crate::DeltaT;
//
// pub const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
//
// #[rustfmt::skip]
// pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.5,
//     0.0, 0.0, 0.0, 1.0,
// );
//
// #[derive(Debug, WinnyResource)]
// pub struct Camera {
//     pub position: Point3<f32>,
//     yaw: Rad<f32>,
//     pitch: Rad<f32>,
//     pub projection: Projection,
// }
//
// impl Camera {
//     pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
//         position: V,
//         yaw: Y,
//         pitch: P,
//         projection: Projection,
//     ) -> Self {
//         Self {
//             position: position.into(),
//             yaw: yaw.into(),
//             pitch: pitch.into(),
//             projection,
//         }
//     }
//
//     pub fn calc_matrix(&self) -> Matrix4<f32> {
//         let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
//         let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
//
//         Matrix4::look_to_rh(
//             self.position,
//             Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
//             Vector3::unit_y(),
//         )
//     }
// }
//
// #[derive(Debug)]
// pub struct Projection {
//     aspect: f32,
//     fovy: Rad<f32>,
//     znear: f32,
//     zfar: f32,
// }
//
// impl Projection {
//     pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
//         Self {
//             aspect: width as f32 / height as f32,
//             fovy: fovy.into(),
//             znear,
//             zfar,
//         }
//     }
//
//     pub fn resize(&mut self, width: u32, height: u32) {
//         self.aspect = width as f32 / height as f32;
//     }
//
//     pub fn calc_matrix(&self) -> Matrix4<f32> {
//         OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
//     }
// }
//
// #[derive(Debug, WinnyResource)]
// pub struct CameraController {
//     pub amount_left: f32,
//     pub amount_right: f32,
//     pub amount_forward: f32,
//     pub amount_backward: f32,
//     pub amount_up: f32,
//     pub amount_down: f32,
//     pub rotate_horizontal: f32,
//     pub rotate_vertical: f32,
//     scroll: f32,
//     speed: f32,
//     sensitivity: f32,
// }
//
// pub struct Camera2D;
//
// impl Plugin for Camera2D {
//     fn build(&self, _world: &mut ecs::World, _scheduler: &mut ecs::Scheduler) {
//         // let projection = Projection::new(
//         //     renderer.config.width,
//         //     renderer.config.height,
//         //     cgmath::Deg(45.0),
//         //     0.1,
//         //     100.0,
//         // );
//
//         // world.insert_resource(Camera::new(
//         //     (0.0, 0.0, 0.0),
//         //     cgmath::Deg(-90.0),
//         //     cgmath::Deg(-20.0),
//         //     projection,
//         // ));
//     }
// }
//
// impl CameraController {
//     pub fn new(speed: f32, sensitivity: f32) -> Self {
//         Self {
//             amount_left: 0.0,
//             amount_right: 0.0,
//             amount_forward: 0.0,
//             amount_backward: 0.0,
//             amount_up: 0.0,
//             amount_down: 0.0,
//             rotate_horizontal: 0.0,
//             rotate_vertical: 0.0,
//             scroll: 0.0,
//             speed,
//             sensitivity,
//         }
//     }
//
//     pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
//         self.rotate_horizontal = mouse_dx as f32;
//         self.rotate_vertical = mouse_dy as f32;
//     }
//
//     pub fn update_camera(&mut self, camera: &mut Camera, dt: &DeltaT) -> () {
//         let dt = dt.0 as f32;
//
//         // Move forward/backward and left/right
//         let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
//         let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
//         let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
//         camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
//         camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;
//
//         // Move in/out (aka. "zoom")
//         // Note: this isn't an actual zoom. The camera's position
//         // changes when zooming. I've added this to make it easier
//         // to get closer to an object you want to focus on.
//         let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
//         let scrollward =
//             Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
//         camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
//         self.scroll = 0.0;
//
//         // Move up/down. Since we don't use roll, we can just
//         // modify the y coordinate directly.
//         camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;
//
//         // Rotate
//         camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
//         camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;
//
//         // If process_mouse isn't called every frame, these values
//         // will not get set to zero, and the camera will rotate
//         // when moving in a non-cardinal direction.
//         self.rotate_horizontal = 0.0;
//         self.rotate_vertical = 0.0;
//
//         // Keep the camera's angle from going too high/low.
//         if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
//             camera.pitch = -Rad(SAFE_FRAC_PI_2);
//         } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
//             camera.pitch = Rad(SAFE_FRAC_PI_2);
//         }
//     }
// }

// #[repr(C)]
// #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct CameraUniform {
//     view_position: [f32; 4],
//     view_proj: [[f32; 4]; 4],
// }
//
// impl CameraUniform {
//     pub fn new() -> Self {
//         Self {
//             view_position: [0.0; 4],
//             view_proj: [[0.0; 4]; 4], //cgmath::Matrix4::identity().into(),
//         }
//     }
//
//     //pub fn update_view_proj(&mut self, camera: &Camera) {
//     // self.view_position = camera.position.to_homogeneous().into();
//     // self.view_proj = (camera.projection.calc_matrix() * camera.calc_matrix()).into();
//     // }
// }
