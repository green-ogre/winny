use app::{
    plugins::Plugin,
    window::{ViewPort, Window},
};
use ecs::{prelude::*, WinnyBundle, WinnyComponent};
use render::{RenderBindGroup, RenderBuffer, RenderDevice, RenderLayer, RenderPass, RenderQueue};
use winny_math::{
    matrix::Matrix4x4f,
    quaternion::Quaternion,
    vector::{Vec3f, Vec4f},
};

use wgpu::util::DeviceExt;

use crate::Transform;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_systems(
            ecs::Schedule::PreRender,
            (generate_camera_bindings, update_camera_view),
        )
        .add_systems(ecs::Schedule::Render, render_cameras);
    }
}

#[derive(WinnyBundle)]
pub struct CameraBundle2d {
    pub camera: Camera,
    pub projection: Projection,
    pub transform: Transform,
    pub render_layer: RenderLayer,
}

impl Default for CameraBundle2d {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            projection: Projection::Orthographic(OrthographicProjection::default()),
            transform: Transform::default(),
            render_layer: RenderLayer(0),
        }
    }
}

#[derive(WinnyBundle)]
pub struct CameraBundle3d {
    pub camera: Camera,
    pub projection: Projection,
    pub transform: Transform,
    pub render_layer: RenderLayer,
}

impl Default for CameraBundle3d {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            projection: Projection::Perspective(PerspectiveProjection::default()),
            transform: Transform::default(),
            render_layer: RenderLayer(0),
        }
    }
}

#[derive(WinnyComponent)]
pub struct Camera {
    pub is_visible: bool,
    pub view_port: Option<ViewPort>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            is_visible: true,
            view_port: None,
        }
    }
}

#[derive(WinnyComponent, Debug)]
pub enum Projection {
    Orthographic(OrthographicProjection),
    Perspective(PerspectiveProjection),
}

impl Projection {
    pub fn matrix(&mut self, view_port: &ViewPort) -> Matrix4x4f {
        match self {
            Self::Orthographic(p) => p.projection(view_port),
            Self::Perspective(p) => p.projection(view_port),
        }
    }
}

#[derive(Debug)]
pub struct PerspectiveProjection {
    fov: f32,
    aspect: f32,
    near: f32,
    far: f32,
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        Self {
            fov: 90.,
            aspect: 0.0,
            near: 0.0,
            far: 1000.0,
        }
    }
}

impl PerspectiveProjection {
    pub fn new(viewport: &ViewPort, fov: f32, near: f32, far: f32) -> Self {
        let aspect = (viewport.max.x - viewport.min.x) / (viewport.max.y - viewport.max.x);

        Self {
            fov,
            near,
            far,
            aspect,
        }
    }

    pub fn projection(&mut self, viewport: &ViewPort) -> Matrix4x4f {
        let mut output = Matrix4x4f::zero();

        self.aspect = (viewport.max.x - viewport.min.x) / (viewport.max.y - viewport.max.x);
        let fov = (self.fov / 2.).tan();
        output.m[0][0] = 1. / (self.aspect * fov); // 1 / (tan(FOV / 2) * AspectRatio)
        output.m[1][1] = 1. / fov; // 1 / tan(FOV / 2)
        output.m[2][2] = (self.far + self.near) / (self.near - self.far); // (Far + Near) / (Near - Far)
        output.m[2][3] = (2. * self.far * self.near) / (self.near - self.far); // (2 * Far * Near) / (Near - Far)
        output.m[3][2] = -1.;

        output
    }
}

#[derive(Debug)]
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

    pub fn projection(&self, viewport: &ViewPort) -> Matrix4x4f {
        // TODO: fix
        let (top, left) = (viewport.min.y, viewport.min.x);
        let (bottom, right) = (viewport.max.x, viewport.max.y);
        Matrix4x4f {
            m: [
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
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: Vec4f,
    view_proj: Matrix4x4f,
}

impl CameraUniform {
    pub fn new(
        viewport: &ViewPort,
        mut projection: &mut Projection,
        transform: &Transform,
    ) -> Self {
        let view_proj = match &mut projection {
            Projection::Orthographic(OrthographicProjection {
                view_port,
                far,
                near,
            }) => {
                unimplemented!()
            }
            Projection::Perspective(p) => {
                p.projection(viewport) * transform.transformation_matrix()
            }
        };
        let view_position = Vec4f::to_homogenous(transform.translation);

        Self {
            view_position,
            view_proj,
        }
    }
}

fn update_camera_view(
    mut cameras: Query<(Camera, Mut<Projection>, Transform, RenderBuffer)>,
    queue: Res<RenderQueue>,
    window: Res<Window>,
) {
    for (camera, mut projection, transform, buffer) in cameras.iter_mut() {
        let viewport = if let Some(viewport) = &camera.view_port {
            viewport
        } else {
            &window.viewport
        };

        let uniform = CameraUniform::new(viewport, projection, transform);
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[uniform]));
    }
}

fn generate_camera_bindings(
    mut commands: Commands,
    device: Res<RenderDevice>,
    mut camera_bundles: Query<
        (Entity, Camera, Mut<Projection>, Transform),
        Without<RenderBindGroup>,
    >,
    window: Res<Window>,
) {
    for (entity, camera, mut projection, transform) in camera_bundles.iter_mut() {
        util::tracing::error!("CAMERA");
        let viewport = if let Some(viewport) = &camera.view_port {
            viewport
        } else {
            &window.viewport
        };

        let camera_uniform = CameraUniform::new(viewport, projection, transform);
        util::tracing::info!(
            "generating camera binding: {entity:?}, {projection:?}, {transform:?}"
        );
        let uniform_buffer = new_camera_buffer(&device, camera_uniform);
        let uniform_bind_group_layout = new_camera_bind_group_layout(&device);
        let uniform_bind_group =
            new_camera_bind_group(&device, &uniform_bind_group_layout, &uniform_buffer);
        commands.get_entity(entity).insert((
            RenderBuffer(uniform_buffer),
            RenderBindGroup(uniform_bind_group),
        ));
    }
}

fn render_cameras(
    cameras: Query<(Camera, Projection, RenderLayer)>,
    render_pass: Query<(RenderPass, RenderLayer)>,
) {
    // for (camera, projection, layer) in cameras.iter() {
    //     for (pass, _) in render_pass.iter().filter(|(_, l)| *l == layer) {
    //         // pass.run();
    //     }
    // }
}

fn new_camera_buffer(device: &RenderDevice, camera_uniform: CameraUniform) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("camera"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

pub fn new_camera_bind_group_layout(device: &RenderDevice) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("camera"),
    })
}

fn new_camera_bind_group(
    device: &RenderDevice,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera"),
    })
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
