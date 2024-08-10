use ecs::WinnyAsEgui;

#[derive(WinnyAsEgui, Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Radf(pub f32);

impl From<Degrees> for Radf {
    fn from(value: Degrees) -> Self {
        Radf(value.0 / 180. * std::f32::consts::PI)
    }
}

#[derive(WinnyAsEgui, Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

impl From<Radf> for Degrees {
    fn from(value: Radf) -> Self {
        Degrees(value.0 * 180. / std::f32::consts::PI)
    }
}
