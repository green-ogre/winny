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

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: [[0.0; 4]; 4], //cgmath::Matrix4::identity().into(),
        }
    }

    //pub fn update_view_proj(&mut self, camera: &Camera) {
    // self.view_position = camera.position.to_homogeneous().into();
    // self.view_proj = (camera.projection.calc_matrix() * camera.calc_matrix()).into();
    // }
}
