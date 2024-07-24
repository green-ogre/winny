use std::f32::consts::PI;

use self::prelude::Vec2f;

use super::*;

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
