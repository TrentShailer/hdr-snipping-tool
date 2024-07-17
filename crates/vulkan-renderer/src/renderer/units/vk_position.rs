use super::{
    from_physical::{AddPhysical, FromPhysical},
    vk_scale::VkSize,
};

/// Represents a position in vulkan coordinate space `(-1.0, -1.0), (1.0, 1.0)`
///
/// Objects are positioned by their centre
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkPosition {
    pub x: f64,
    pub y: f64,
}

impl VkPosition {
    pub fn get_center(top_left: VkPosition, size: VkSize) -> VkPosition {
        VkPosition::from([top_left.x + size.x / 2.0, top_left.y + size.y / 2.0])
    }

    pub fn as_f32_array(&self) -> [f32; 2] {
        [self.x as f32, self.y as f32]
    }

    pub fn as_f64_array(&self) -> [f64; 2] {
        [self.x, self.y]
    }
}

impl FromPhysical<u32> for VkPosition {
    fn from_physical(value: [u32; 2], screen_size: [u32; 2]) -> Self {
        let value: [f64; 2] = [value[0].into(), value[1].into()];
        Self::from_physical(value, screen_size)
    }
}

impl FromPhysical<f32> for VkPosition {
    fn from_physical(value: [f32; 2], screen_size: [u32; 2]) -> Self {
        let value: [f64; 2] = [value[0].into(), value[1].into()];
        Self::from_physical(value, screen_size)
    }
}

impl FromPhysical<f64> for VkPosition {
    fn from_physical(value: [f64; 2], screen_size: [u32; 2]) -> Self {
        let screen_size: [f64; 2] = [screen_size[0].into(), screen_size[1].into()];

        // Scale value between 0 and 1
        let x = value[0] / screen_size[0];
        let y = value[1] / screen_size[1];

        // Scale value between -1 and 1
        let x = x * 2.0 - 1.0;
        let y = y * 2.0 - 1.0;

        Self { x, y }
    }
}

impl From<[f32; 2]> for VkPosition {
    fn from(value: [f32; 2]) -> Self {
        Self {
            x: value[0].into(),
            y: value[1].into(),
        }
    }
}

impl From<[f64; 2]> for VkPosition {
    fn from(value: [f64; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
        }
    }
}

impl AddPhysical<u32> for VkPosition {
    fn add_physical(self, value: [u32; 2], screen_size: [u32; 2]) -> Self {
        let offset = VkSize::from_physical(value, screen_size);

        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
        }
    }
}

impl AddPhysical<f32> for VkPosition {
    fn add_physical(self, value: [f32; 2], screen_size: [u32; 2]) -> Self {
        let offset = VkSize::from_physical(value, screen_size);

        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
        }
    }
}

impl AddPhysical<f64> for VkPosition {
    fn add_physical(self, value: [f64; 2], screen_size: [u32; 2]) -> Self {
        let offset = VkSize::from_physical(value, screen_size);

        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
        }
    }
}
