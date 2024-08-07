use crate::{matrix::Matrix4x4f, vector::Vec3f};

#[derive(Debug, Clone, Copy)]
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

    // from cgmath
    pub fn rotation_matrix(&self) -> Matrix4x4f {
        let x2 = self.v.x + self.v.x;
        let y2 = self.v.y + self.v.y;
        let z2 = self.v.z + self.v.z;

        let xx2 = x2 * self.v.x;
        let xy2 = x2 * self.v.y;
        let xz2 = x2 * self.v.z;

        let yy2 = y2 * self.v.y;
        let yz2 = y2 * self.v.z;
        let zz2 = z2 * self.v.z;

        let sy2 = y2 * self.s;
        let sz2 = z2 * self.s;
        let sx2 = x2 * self.s;

        #[cfg_attr(rustfmt, rustfmt_skip)]
        Matrix4x4f {
            m: [
                [1. - yy2 - zz2, xy2 + sz2,      xz2 - sy2,      0.],
                [xy2 - sz2,      1. - xx2 - zz2, yz2 + sx2,      0.],
                [xz2 + sy2,      yz2 - sx2,      1. - xx2 - yy2, 0.],
                [0.,             0.,             0.,             1.],
            ]
        }
    }
}
