use std::{f32::consts::PI, ops::Mul};

use self::{angle::Radf, prelude::Vec2f, vector::Vec4f};

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
            v: [
                self.v[0] * rhs.m[0][0] + self.v[1] * rhs.m[0][1],
                self.v[0] * rhs.m[1][0] + self.v[1] * rhs.m[1][1],
            ],
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
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
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

impl Mul<Vec4f> for Matrix4x4f {
    type Output = Vec4f;
    fn mul(self, rhs: Vec4f) -> Self::Output {
        let mut output = Vec4f::zero();
        for y in 0..4 {
            let mut dot = 0.;
            for x in 0..4 {
                dot += rhs.v[x] * self.m[y][x];
            }
            output.v[y] = dot;
        }

        output
    }
}

impl Matrix4x4f {
    pub fn zero() -> Self {
        Self { m: [[0.; 4]; 4] }
    }
}

pub fn world_to_screen_space_matrix4x4f(
    screen_width: f32,
    screen_height: f32,
    max_z: f32,
) -> Matrix4x4f {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    Matrix4x4f {
        m: [
            [2. / screen_width, 0.,                  0.,         0.],
            [0.,                -2. / screen_height, 0.,         0.],
            [0.,                0.,                  1. / max_z, 0.],
            [0.,                0.,                  0.,         1.],
        ]
    }
}

pub fn translation_matrix4x4f(point: Vec4f) -> Matrix4x4f {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    Matrix4x4f {
        m: [
            [1., 0., 0., point.v[0]],
            [0., 1., 0., point.v[1]],
            [0., 0., 1., point.v[2]],
            [0., 0., 0., 1.        ],
        ]
    }
}

// Takes normalized width and height
pub fn rotation_2d_matrix4x4f(theta: impl Into<Radf>) -> Matrix4x4f {
    let theta = theta.into().0;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    Matrix4x4f {
        m: [
            [theta.cos(), (-theta).sin(), 0., 0.],
            [theta.sin(), theta.cos(),    0., 0.],
            [0.,          0.,             1., 0.],
            [0.,          0.,             0., 1.],
        ]
    }
}

pub fn scale_matrix4x4f(scale: Vec2f) -> Matrix4x4f {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    Matrix4x4f {
        m: [
            [scale.v[0], 0.,         0., 0.],
            [0.,         scale.v[1], 0., 0.],
            [0.,         0.,         1., 0.],
            [0.,         0.,         0., 1.],
        ]
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
