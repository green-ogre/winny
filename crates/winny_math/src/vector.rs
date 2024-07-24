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

#[derive(Debug, Clone, Copy)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

impl From<[f32; 2]> for Vec2f {
    fn from(value: [f32; 2]) -> Self {
        Self::new(value[0], value[1])
    }
}

impl Vec2f {
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn as_matrix(&self) -> [f32; 2] {
        [self.x, self.y]
    }

    pub fn distance(&self, other: &Vec2f) -> f32 {
        let x = self.x - other.x;
        let y = self.y - other.y;
        (x * x + y * y).sqrt()
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

impl std::ops::Add<Vec2f> for Vec2f {
    type Output = Vec2f;

    fn add(self, _rhs: Vec2f) -> Vec2f {
        Vec2f {
            x: self.x + _rhs.x,
            y: self.y + _rhs.y,
        }
    }
}

impl std::ops::AddAssign<Vec2f> for Vec2f {
    fn add_assign(&mut self, rhs: Vec2f) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Sub<Vec2f> for Vec2f {
    type Output = Vec2f;

    fn sub(self, _rhs: Vec2f) -> Vec2f {
        Vec2f {
            x: self.x - _rhs.x,
            y: self.y - _rhs.y,
        }
    }
}

impl std::ops::SubAssign<Vec2f> for Vec2f {
    fn sub_assign(&mut self, _rhs: Vec2f) {
        self.x -= _rhs.x;
        self.y -= _rhs.y;
    }
}

impl std::ops::Mul<f32> for Vec2f {
    type Output = Vec2f;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec2f {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl std::ops::MulAssign<f32> for Vec2f {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl std::ops::Div<f32> for Vec2f {
    type Output = Vec2f;

    fn div(self, rhs: f32) -> Self::Output {
        Vec2f {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl std::ops::DivAssign<f32> for Vec2f {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

#[derive(Debug, Clone, Copy)]
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

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn as_matrix(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    // pub fn distance(&self, other: &Vec3f) -> f32 {
    //     let x = self.x - other.x;
    //     let y = self.y - other.y;
    //     (x * x + y * y).sqrt()
    // }
    //
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
