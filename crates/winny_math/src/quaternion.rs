use crate::{prelude::Matrix4x4f, vector::Vec3f};

#[derive(Debug)]
pub struct Quaternion {
    pub v: Vec3f,
    pub s: f32,
}

impl Quaternion {
    pub fn zero() -> Self {
        Self {
            v: Vec3f::zero(),
            s: 1.0,
        }
    }

    pub fn rotation_matrix(&self) -> Matrix4x4f {
        let (x, y, z, s) = (self.v.x, self.v.y, self.v.z, self.s);
        let (x2, y2, z2) = (x * x, y * y, z * z);

        Matrix4x4f {
            m: [
                [
                    1. - 2. * y2 - 2. * z2,
                    2. * x * y - 2. * z * s,
                    2. * x * z + 2. * y * s,
                    0.,
                ],
                [
                    2. * x * y + 2. * z * s,
                    1. - 2. * x2 - 2. * z2,
                    2. * y * z - 2. * x * s,
                    0.,
                ],
                [
                    2. * x * z - 2. * y * s,
                    2. * y * z + 2. * x * s,
                    1. - 2. * x2 - 2. * y2,
                    0.,
                ],
                [0., 0., 0., 1.],
            ],
        }
    }
}
