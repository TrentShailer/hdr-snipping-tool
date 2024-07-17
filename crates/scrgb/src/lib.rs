use derive_more::{Add, AddAssign, Display, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use windows::Win32::Graphics::Direct2D::D2D1_SCENE_REFERRED_SDR_WHITE_LEVEL;

/// A value in the scRGB color space.
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    PartialOrd,
    Neg,
    Add,
    Sub,
    Mul,
    Div,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    Display,
)]
pub struct ScRGB(pub f32);

impl ScRGB {
    pub fn from_nits(nits: f32) -> Self {
        Self(nits / D2D1_SCENE_REFERRED_SDR_WHITE_LEVEL)
    }

    pub fn as_nits(&self) -> f32 {
        self.0 * D2D1_SCENE_REFERRED_SDR_WHITE_LEVEL
    }

    pub fn as_nits_string(&self) -> String {
        let nits = self.as_nits().round() as i32;

        nits.to_string()
    }
}
