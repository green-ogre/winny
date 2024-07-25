use std::{f32::consts::PI, ops::Mul};

use self::{prelude::Vec2f, vector::Vec3f};

use super::*;

#[derive(Debug)]
pub struct Matrix2x2f {
    pub m: [[f32; 2]; 2],
}

impl Into<Matrix2x2f> for [f32; 4] {
    fn into(self) -> Matrix2x2f {
        Matrix2x2f {
            m: [[self[0], self[1]], [self[2], self[3]]],
        }
    }
}

impl Matrix2x2f {
    pub fn rotation_2d(vec: Vec2f, theta: f32) -> Vec2f {
        let theta = theta * PI / 180.0;
        let rm: Matrix2x2f = [theta.cos(), -theta.sin(), theta.sin(), theta.cos()].into();

        vec * rm
    }
}

impl std::ops::Mul<Matrix2x2f> for Vec2f {
    type Output = Vec2f;

    fn mul(self, rhs: Matrix2x2f) -> Self::Output {
        Vec2f {
            x: self.x * rhs.m[0][0] + self.y * rhs.m[0][1],
            y: self.x * rhs.m[1][0] + self.y * rhs.m[1][1],
        }
    }
}

#[derive(Debug)]
pub struct Matrix3x3f {
    pub m: [[f32; 3]; 3],
}

impl Mul<Matrix3x3f> for Matrix3x3f {
    type Output = Matrix3x3f;

    fn mul(self, rhs: Matrix3x3f) -> Self::Output {
        let mut output = Matrix3x3f::zero();

        for y in 0..3 {
            for x_offset in 0..3 {
                let mut dot = 0.0;
                for i in 0..3 {
                    let a = self.m[y][i];
                    let b = rhs.m[i][x_offset];
                    dot += a * b;
                }
                output.m[y][x_offset] = dot;
            }
        }

        output
    }
}

impl Matrix3x3f {
    pub fn zero() -> Self {
        Self { m: [[0.; 3]; 3] }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Matrix4x4f {
    pub m: [[f32; 4]; 4],
}

impl Mul<Matrix4x4f> for Matrix4x4f {
    type Output = Matrix4x4f;

    fn mul(self, rhs: Matrix4x4f) -> Self::Output {
        let mut output = Matrix4x4f::zero();

        for y in 0..4 {
            for x_offset in 0..4 {
                let mut dot = 0.0;
                for i in 0..4 {
                    let a = self.m[y][i];
                    let b = rhs.m[i][x_offset];
                    dot += a * b;
                }
                output.m[y][x_offset] = dot;
            }
        }

        output
    }
}

impl Matrix4x4f {
    pub fn zero() -> Self {
        Self { m: [[0.; 4]; 4] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrices() {
        let m1 = Matrix3x3f {
            m: [[1.0, 2.0, 3.2], [2.2, 8.0, 2.2], [8.0, 0.0, 1.0]],
        };

        let m2 = Matrix3x3f {
            m: [[1.7, 3.0, 1.0], [0.2, 2.0, 5.0], [3.7, 1.0, 0.0]],
        };

        let m3 = m1 * m2;
        println!("{m3:?}");
    }
}
