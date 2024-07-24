use app::{plugins::Plugin, window::ViewPort};
use ecs::{Query, WinnyBundle, WinnyComponent};
use render::{RenderLayer, RenderPass};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_systems(ecs::Schedule::Render, render_cameras);
    }
}

#[derive(WinnyBundle)]
pub struct CameraBundle2d {
    pub camera: Camera,
    pub projection: Projection,
    pub render_layer: RenderLayer,
}

impl Default for CameraBundle2d {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            projection: Projection::Orthographic(OrthographicProjection::default()),
            render_layer: RenderLayer(0),
        }
    }
}

#[derive(WinnyComponent)]
pub struct Camera {
    is_visible: bool,
    view_port: Option<ViewPort>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            is_visible: true,
            view_port: None,
        }
    }
}

#[derive(WinnyComponent)]
pub enum Projection {
    Orthographic(OrthographicProjection),
    Perspective,
}

impl Projection {
    pub fn transform_mat4x4(&self, view_port: &ViewPort) -> [[f32; 4]; 4] {
        match self {
            Self::Orthographic(p) => p.transform_mat4x4(view_port),
            Self::Perspective => todo!(),
        }
    }
}

// TODO: just have it be the enum
pub struct OrthographicProjection {
    view_port: Option<ViewPort>,
    far: f32,
    near: f32,
}

impl Default for OrthographicProjection {
    fn default() -> Self {
        Self {
            view_port: None,
            far: 1000.0,
            near: 0.0,
        }
    }
}

impl OrthographicProjection {
    pub fn new(far: f32, near: f32) -> Self {
        Self {
            view_port: None,
            far,
            near,
        }
    }

    pub fn transform_mat4x4(&self, viewport: &ViewPort) -> [[f32; 4]; 4] {
        let (top, left) = (viewport.top_left.y, viewport.top_left.x);
        let (bottom, right) = (top + viewport.height, left + viewport.width);
        [
            [
                2.0 / (right - left),
                0.0,
                0.0,
                -((right + left) / (right - left)),
            ],
            [
                0.0,
                2.0 / (top - bottom),
                0.0,
                -((top + bottom) / (top - bottom)),
            ],
            [
                0.0,
                0.0,
                -2.0 / (self.far - self.near),
                -((self.far + self.near) / (self.far - self.near)),
            ],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }
}

fn render_cameras(
    cameras: Query<(Camera, Projection, RenderLayer)>,
    render_pass: Query<(RenderPass, RenderLayer)>,
) {
    for (camera, projection, layer) in cameras.iter() {
        for (pass, _) in render_pass.iter().filter(|(_, l)| *l == layer) {
            // pass.run();
        }
    }
}

// use app::input::mouse_and_key::{KeyCode, KeyInput, MouseInput};
// use app::plugins::Plugin;
// use app::time::DeltaTime;
// use app::window::winit::event::ElementState;
// use ecs::{EventReader, ResMut};
// use winny_math::vector::Vec2f;
//
// use cgmath::*;
// use std::f32::consts::FRAC_PI_2;
// use winit::dpi::PhysicalPosition;
// use winit::event::*;
//
// pub struct Camera3DPlugin;
//
// impl Plugin for Camera3DPlugin {
//     fn build(&mut self, app: &mut app::app::App) {
//         app.insert_resource(Camera3D::new(width, height, position, yaw, pitch))
//     }
// }
//
// pub struct Camera2D {
//     width: f32,
//     height: f32,
//     offset: Vec2f,
// }
//
// impl Camera2D {
//     pub fn new(width: f32, height: f32, offset: Vec2f) -> Self {
//         Self {
//             width,
//             height,
//             offset,
//         }
//     }
//
//     pub fn width(&self) -> f32 {
//         self.width
//     }
//
//     pub fn height(&self) -> f32 {
//         self.height
//     }
//
//     pub fn offset(&self) -> f32 {
//         self.height
//     }
// }
//
// #[rustfmt::skip]
// pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.5,
//     0.0, 0.0, 0.0, 1.0,
// );
//
// const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
//
// #[derive(Debug)]
// pub struct Camera3D {
//     width: f32,
//     height: f32,
//     position: Point3<f32>,
//     yaw: Rad<f32>,
//     pitch: Rad<f32>,
// }
//
// impl Camera3D {
//     pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
//         width: f32,
//         height: f32,
//         position: V,
//         yaw: Y,
//         pitch: P,
//     ) -> Self {
//         Self {
//             width,
//             height,
//             position: position.into(),
//             yaw: yaw.into(),
//             pitch: pitch.into(),
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
// pub struct Camera3DController {
//     amount_left: f32,
//     amount_right: f32,
//     amount_forward: f32,
//     amount_backward: f32,
//     amount_up: f32,
//     amount_down: f32,
//     rotate_horizontal: f32,
//     rotate_vertical: f32,
//     scroll: f32,
//     speed: f32,
//     sensitivity: f32,
// }
//
// impl Camera3DController {
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
//     pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
//         let amount = if state == ElementState::Pressed {
//             1.0
//         } else {
//             0.0
//         };
//         match key {
//             KeyCode::W | KeyCode::Up => {
//                 self.amount_forward = amount;
//                 true
//             }
//             KeyCode::S | KeyCode::Down => {
//                 self.amount_backward = amount;
//                 true
//             }
//             KeyCode::A | KeyCode::Left => {
//                 self.amount_left = amount;
//                 true
//             }
//             KeyCode::D | KeyCode::Right => {
//                 self.amount_right = amount;
//                 true
//             }
//             KeyCode::Space => {
//                 self.amount_up = amount;
//                 true
//             }
//             KeyCode::LShift => {
//                 self.amount_down = amount;
//                 true
//             }
//             _ => false,
//         }
//     }
//
//     pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
//         self.rotate_horizontal = mouse_dx as f32;
//         self.rotate_vertical = mouse_dy as f32;
//     }
//
//     pub fn update_camera(&mut self, camera: &mut Camera3D, dt: DeltaTime) {
//         let dt = *dt;
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
//
// fn update_camera_controller_3d(
//     mut camera_controller: Option<ResMut<Camera3DController>>,
//     keyboard_events: EventReader<KeyInput>,
//     mouse_events: EventReader<MouseInput>,
// ) {
//     let Some(camera_controller) = &mut camera_controller else {
//         return;
//     };
//
//     for k in keyboard_events.peak_read() {
//         camera_controller.process_keyboard(k.code, k.state);
//     }
//
//     for m in mouse_events.peak_read() {
//         camera_controller.process_mouse(m.dx, m.dy);
//     }
// }
//
// fn update_camera_3d(
//     mut camera: ResMut<Camera3D>,
//     camera_controller: Option<Res<Camera3DController>>,
// ) {
//     let Some(camera_controller) = camera_controller else {
//         return;
//     };
//
//     camera_controller.update_camera(camera, dt)
// }
