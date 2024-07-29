use std::ops::{Add, AddAssign, SubAssign};

use crate::prelude::Matrix4x4f;

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Default)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

impl Vec2 {
    pub fn new(x: i32, y: i32) -> Self {
        Vec2 { x, y }
    }

    pub fn zero() -> Self {
        Vec2 { x: 0, y: 0 }
    }

    pub fn x(dist: i32) -> Self {
        Vec2 { x: dist, y: 0 }
    }

    pub fn y(dist: i32) -> Self {
        Vec2 { x: 0, y: dist }
    }
    //
    // pub fn distance(&self, other: &Vec2) -> f32 {
    //     let x = (self.x - other.x) as f32;
    //     let y = (self.y - other.y) as f32;
    //     (x * x + y * y).sqrt()
    // }
}

impl std::ops::Add<Vec2> for Vec2 {
    type Output = Vec2;

    fn add(self, _rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + _rhs.x,
            y: self.y + _rhs.y,
        }
    }
}

impl std::ops::Sub<Vec2> for Vec2 {
    type Output = Vec2;

    fn sub(self, _rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - _rhs.x,
            y: self.y - _rhs.y,
        }
    }
}

impl std::ops::AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec2f {
    pub v: [f32; 2],
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

impl Vec2f {
    pub fn zero() -> Self {
        Self { v: [0.0, 0.0] }
    }

    pub fn one() -> Self {
        Self { v: [1.; 2] }
    }

    pub fn new(x: f32, y: f32) -> Self {
        Self { v: [x, y] }
    }

    pub fn as_matrix(&self) -> [f32; 2] {
        self.v
    }

    // pub fn distance(&self, other: &Vec2f) -> f32 {
    //     let x = self.x - other.x;
    //     let y = self.y - other.y;
    //     (x * x + y * y).sqrt()
    // }

    pub fn is_zero(&self) -> bool {
        self.v[0] == 0.0 && self.v[1] == 0.0
    }

    pub fn normalize(&self) -> Vec2f {
        let m = self.magnitude();

        Vec2f {
            v: [self.v[0] / m, self.v[1] / m],
        }
    }

    pub fn magnitude(&self) -> f32 {
        (self.v[0] * self.v[0] + self.v[1] * self.v[1]).sqrt()
    }
}

// impl std::ops::Add<Vec2f> for Vec2f {
//     type Output = Vec2f;
//
//     fn add(self, _rhs: Vec2f) -> Vec2f {
//         Vec2f {
//             x: self.x + _rhs.x,
//             y: self.y + _rhs.y,
//         }
//     }
// }
//
// impl std::ops::AddAssign<Vec2f> for Vec2f {
//     fn add_assign(&mut self, rhs: Vec2f) {
//         self.x += rhs.x;
//         self.y += rhs.y;
//     }
// }
//
// impl std::ops::Sub<Vec2f> for Vec2f {
//     type Output = Vec2f;
//
//     fn sub(self, _rhs: Vec2f) -> Vec2f {
//         Vec2f {
//             x: self.x - _rhs.x,
//             y: self.y - _rhs.y,
//         }
//     }
// }
//
// impl std::ops::SubAssign<Vec2f> for Vec2f {
//     fn sub_assign(&mut self, _rhs: Vec2f) {
//         self.x -= _rhs.x;
//         self.y -= _rhs.y;
//     }
// }
//
// impl std::ops::Mul<f32> for Vec2f {
//     type Output = Vec2f;
//
//     fn mul(self, rhs: f32) -> Self::Output {
//         Vec2f {
//             x: self.x * rhs,
//             y: self.y * rhs,
//         }
//     }
// }
//
// impl std::ops::MulAssign<f32> for Vec2f {
//     fn mul_assign(&mut self, rhs: f32) {
//         self.x *= rhs;
//         self.y *= rhs;
//     }
// }
//
// impl std::ops::Div<f32> for Vec2f {
//     type Output = Vec2f;
//
//     fn div(self, rhs: f32) -> Self::Output {
//         Vec2f {
//             x: self.x / rhs,
//             y: self.y / rhs,
//         }
//     }
// }
//
// impl std::ops::DivAssign<f32> for Vec2f {
//     fn div_assign(&mut self, rhs: f32) {
//         self.x /= rhs;
//         self.y /= rhs;
//     }
// }

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
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
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec4f {
    pub v: [f32; 4],
}

impl SubAssign<Vec4f> for Vec4f {
    fn sub_assign(&mut self, rhs: Vec4f) {
        self.v[0] -= rhs.v[0];
        self.v[1] -= rhs.v[1];
        self.v[2] -= rhs.v[2];
        self.v[3] -= rhs.v[3];
    }
}

impl Add<Vec4f> for Vec4f {
    type Output = Vec4f;

    fn add(mut self, rhs: Vec4f) -> Self::Output {
        self.v[0] += rhs.v[0];
        self.v[1] += rhs.v[1];
        self.v[2] += rhs.v[2];
        self.v[3] += rhs.v[3];

        self
    }
}

impl AddAssign<Vec4f> for Vec4f {
    fn add_assign(&mut self, rhs: Vec4f) {
        self.v[0] += rhs.v[0];
        self.v[1] += rhs.v[1];
        self.v[2] += rhs.v[2];
        self.v[3] += rhs.v[3];
    }
}

impl Vec4f {
    pub fn new(x: f32, y: f32, z: f32, r: f32) -> Self {
        Self { v: [x, y, z, r] }
    }

    pub fn zero() -> Self {
        Self { v: [0.; 4] }
    }

    pub fn to_homogenous(v: Vec3f) -> Self {
        Self {
            v: [v.x, v.y, v.z, 1.0],
        }
    }
}

#[cfg(test)]
mod tests {
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
