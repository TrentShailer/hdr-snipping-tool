use super::from_physical::{AddPhysical, FromPhysical};

/// Represents a size in vulkan coordinate space `(0.0, 0.0), (2.0, 2.0)`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkSize {
    pub x: f64,
    pub y: f64,
}

impl FromPhysical<u32> for VkSize {
    fn from_physical(value: [u32; 2], screen_size: [u32; 2]) -> Self {
        let value: [f64; 2] = [value[0].into(), value[1].into()];
        Self::from_physical(value, screen_size)
    }
}

impl FromPhysical<f32> for VkSize {
    fn from_physical(value: [f32; 2], screen_size: [u32; 2]) -> Self {
        let value: [f64; 2] = [value[0].into(), value[1].into()];
        Self::from_physical(value, screen_size)
    }
}

impl FromPhysical<f64> for VkSize {
    fn from_physical(value: [f64; 2], screen_size: [u32; 2]) -> Self {
        let screen_size: [f64; 2] = [screen_size[0].into(), screen_size[1].into()];

        // Scale value between 0 and 1
        let x = value[0] / screen_size[0];
        let y = value[1] / screen_size[1];

        // Scale value between 0 and 2
        let x = x * 2.0;
        let y = y * 2.0;

        Self { x, y }
    }
}

impl Into<[f32; 2]> for VkSize {
    fn into(self) -> [f32; 2] {
        [self.x as f32, self.y as f32]
    }
}

impl From<[f32; 2]> for VkSize {
    fn from(value: [f32; 2]) -> Self {
        Self {
            x: value[0].into(),
            y: value[1].into(),
        }
    }
}

impl Into<[f64; 2]> for VkSize {
    fn into(self) -> [f64; 2] {
        [self.x, self.y]
    }
}

impl From<[f64; 2]> for VkSize {
    fn from(value: [f64; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
        }
    }
}

impl AddPhysical<u32> for VkSize {
    fn add_physical(self, value: [u32; 2], screen_size: [u32; 2]) -> Self {
        let offset = VkSize::from_physical(value, screen_size);

        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
        }
    }
}

impl AddPhysical<f32> for VkSize {
    fn add_physical(self, value: [f32; 2], screen_size: [u32; 2]) -> Self {
        let offset = VkSize::from_physical(value, screen_size);

        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
        }
    }
}

impl AddPhysical<f64> for VkSize {
    fn add_physical(self, value: [f64; 2], screen_size: [u32; 2]) -> Self {
        let offset = VkSize::from_physical(value, screen_size);

        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
        }
    }
}
