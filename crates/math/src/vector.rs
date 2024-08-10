use crate::matrix::Matrix4x4f;
use cereal::{WinnyDeserialize, WinnySerialize};
use ecs::egui_widget::Widget;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[repr(C)]
#[derive(
    WinnySerialize,
    WinnyDeserialize,
    Default,
    Debug,
    Copy,
    Clone,
    bytemuck::Pod,
    bytemuck::Zeroable,
    PartialEq,
)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

impl Widget for Vec2f {
    fn display(&mut self, ui: &mut ecs::egui::Ui) {
        ui.with_layout(
            ecs::egui::Layout::left_to_right(ecs::egui::Align::TOP),
            |ui| {
                ui.label("x: ");
                self.x.display(ui);
                ui.label("y: ");
                self.y.display(ui);
            },
        );
    }
}

impl From<[f32; 2]> for Vec2f {
    fn from(value: [f32; 2]) -> Self {
        Self::new(value[0], value[1])
    }
}

impl From<[usize; 2]> for Vec2f {
    fn from(value: [usize; 2]) -> Self {
        Self::new(value[0] as f32, value[1] as f32)
    }
}

impl Add<Vec2f> for Vec2f {
    type Output = Self;
    fn add(mut self, rhs: Vec2f) -> Self::Output {
        self.x += rhs.x;
        self.y += rhs.y;
        self
    }
}

impl Sub<Vec2f> for Vec2f {
    type Output = Self;
    fn sub(mut self, rhs: Vec2f) -> Self::Output {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self
    }
}

impl Mul<Vec2f> for Vec2f {
    type Output = Self;
    fn mul(mut self, rhs: Vec2f) -> Self::Output {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self
    }
}

impl Vec2f {
    pub fn zero() -> Self {
        Self { x: 0., y: 0. }
    }

    pub fn one() -> Self {
        Self { x: 1., y: 1. }
    }

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dist2(&self, other: &Vec2f) -> f32 {
        let x = self.x - other.x;
        let y = self.y - other.y;
        x * x + y * y
    }

    pub fn as_matrix(&self) -> [f32; 2] {
        [self.x, self.y]
    }

    pub fn is_zero(&self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }

    pub fn normalize(&self) -> Vec2f {
        let m = self.magnitude();

        Vec2f {
            x: self.x / m,
            y: self.y / m,
        }
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

#[repr(C)]
#[derive(
    WinnyDeserialize,
    WinnySerialize,
    Debug,
    Default,
    Copy,
    Clone,
    PartialEq,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
pub struct Vec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Widget for Vec3f {
    fn display(&mut self, ui: &mut ecs::egui::Ui) {
        ui.with_layout(
            ecs::egui::Layout::left_to_right(ecs::egui::Align::Min),
            |ui| {
                ui.label("x: ");
                self.x.display(ui);
                ui.label("y: ");
                self.y.display(ui);
                ui.label("r: ");
                self.z.display(ui);
            },
        );
    }
}

impl Vec3f {
    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn one() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }

    pub fn as_array(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn as_matrix(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    /// The squared, 2D distance.
    pub fn dist2(&self, other: &Vec3f) -> f32 {
        let x = self.x - other.x;
        let y = self.y - other.y;
        x * x + y * y
    }

    pub fn is_zero(&self) -> bool {
        self.x == 0.0 && self.y == 0.0 && self.z == 0.0
    }

    pub fn normalize(&self) -> Vec3f {
        let m = self.magnitude();

        if m == 0.0 {
            return Vec3f::zero();
        }

        Vec3f {
            x: self.x / m,
            y: self.y / m,
            z: self.z / m,
        }
    }

    pub fn scale_matrix(&self) -> Matrix4x4f {
        Matrix4x4f {
            m: [
                [self.x, 0., 0., 0.],
                [0., self.y, 0., 0.],
                [0., 0., self.z, 0.],
                [0., 0., 0., 1.],
            ],
        }
    }

    pub fn translation_matrix(&self) -> Matrix4x4f {
        Matrix4x4f {
            m: [
                [1., 0., 0., self.x],
                [0., 1., 0., self.y],
                [0., 0., 1., self.z],
                [0., 0., 0., 1.],
            ],
        }
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

impl Neg for Vec3f {
    type Output = Vec3f;
    fn neg(mut self) -> Self::Output {
        self.x = -self.x;
        self.y = -self.y;
        self.z = -self.z;
        self
    }
}

impl std::ops::Add<Vec3f> for Vec3f {
    type Output = Vec3f;

    fn add(self, _rhs: Vec3f) -> Vec3f {
        Vec3f {
            x: self.x + _rhs.x,
            y: self.y + _rhs.y,
            z: self.z + _rhs.z,
        }
    }
}

impl std::ops::AddAssign<Vec3f> for Vec3f {
    fn add_assign(&mut self, rhs: Vec3f) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl std::ops::Sub<Vec3f> for Vec3f {
    type Output = Vec3f;

    fn sub(self, _rhs: Vec3f) -> Vec3f {
        Vec3f {
            x: self.x - _rhs.x,
            y: self.y - _rhs.y,
            z: self.z - _rhs.z,
        }
    }
}

impl std::ops::SubAssign<Vec3f> for Vec3f {
    fn sub_assign(&mut self, _rhs: Vec3f) {
        self.x -= _rhs.x;
        self.y -= _rhs.y;
        self.z -= _rhs.z;
    }
}

impl std::ops::Mul<f32> for Vec3f {
    type Output = Vec3f;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec3f {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl std::ops::MulAssign<f32> for Vec3f {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

impl std::ops::Div<f32> for Vec3f {
    type Output = Vec3f;

    fn div(self, rhs: f32) -> Self::Output {
        Vec3f {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl std::ops::DivAssign<f32> for Vec3f {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
        self.z /= rhs;
    }
}

#[repr(C)]
#[derive(
    WinnySerialize,
    WinnyDeserialize,
    Debug,
    Copy,
    Clone,
    Default,
    bytemuck::Pod,
    bytemuck::Zeroable,
    PartialEq,
)]
pub struct Vec4f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Widget for Vec4f {
    fn display(&mut self, ui: &mut ecs::egui::Ui) {
        ui.with_layout(
            ecs::egui::Layout::left_to_right(ecs::egui::Align::Min),
            |ui| {
                ui.label("x: ");
                self.x.display(ui);
                ui.label("y: ");
                self.y.display(ui);
                ui.label("r: ");
                self.z.display(ui);
                ui.label("w: ");
                self.w.display(ui);
            },
        );
    }
}

impl From<[f32; 4]> for Vec4f {
    fn from(value: [f32; 4]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
            w: value[3],
        }
    }
}

impl From<Vec4f> for Vec3f {
    fn from(value: Vec4f) -> Self {
        Vec3f::new(value.x, value.y, value.z)
    }
}

impl SubAssign<Vec4f> for Vec4f {
    fn sub_assign(&mut self, rhs: Vec4f) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
        self.w -= rhs.w;
    }
}

impl Add<Vec4f> for Vec4f {
    type Output = Vec4f;

    fn add(mut self, rhs: Vec4f) -> Self::Output {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
        self.w += rhs.w;

        self
    }
}

impl AddAssign<Vec4f> for Vec4f {
    fn add_assign(&mut self, rhs: Vec4f) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
        self.w += rhs.w;
    }
}

impl Vec4f {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn zero() -> Self {
        Self {
            x: 0.,
            y: 0.,
            z: 0.,
            w: 0.,
        }
    }

    pub fn dist2(&self, other: &Vec4f) -> f32 {
        let x = self.x - other.x;
        let y = self.y - other.y;
        let z = self.z - other.z;
        let w = self.w - other.w;
        x * x + y * y + z * z + w * w
    }

    pub fn to_homogenous(v: Vec3f) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w: 1.,
        }
    }

    pub fn is_homogenous(&self) -> bool {
        self.w == 1.0
    }

    pub fn normalize(&self) -> Vec4f {
        let m = self.magnitude();

        Vec4f {
            x: self.x / m,
            y: self.y / m,
            z: self.z / m,
            w: self.w / m,
        }
    }

    pub fn normalize_homogenous(&self) -> Vec4f {
        let m = self.magnitude();

        Vec4f {
            x: self.x / m,
            y: self.y / m,
            z: self.z / m,
            w: 1.,
        }
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }

    pub fn as_matrix(&self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

#[cfg(test)]
mod tests {
    use crate::matrix::Matrix4x4f;

    use super::*;

    #[test]
    fn vector4() {
        let m = Matrix4x4f {
            m: [
                [1.0, 2.0, 3.2, 3.0],
                [2.2, 8.0, 2.2, 8.0],
                [8.0, 0.0, 1.0, 2.0],
                [3.0, 6.0, 1.0, 2.0],
            ],
        };
        let v1 = Vec4f::new(1., 2., 0., 1.);

        let v2 = m * v1;
        println!("{v2:?}");
    }
}
