#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalScale {
    pub x: f32,
    pub y: f32,
}

impl LogicalPosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn from_u32x2(value: [u32; 2], screen_size: [u32; 2]) -> Self {
        Self::from_f32x2([value[0] as f32, value[1] as f32], screen_size)
    }

    pub fn from_f32x2(value: [f32; 2], screen_size: [u32; 2]) -> Self {
        // Scale value between 0 and 1
        let x = value[0] / screen_size[0] as f32;
        let y = value[1] / screen_size[1] as f32;

        // Scale value between -1 and 1
        let x = x * 2.0 - 1.0;
        let y = y * 2.0 - 1.0;

        Self { x, y }
    }

    pub fn add_i32x2(&self, value: [i32; 2], screen_size: [u32; 2]) -> Self {
        self.add_f32x2([value[0] as f32, value[1] as f32], screen_size)
    }

    pub fn add_f32x2(&self, value: [f32; 2], screen_size: [u32; 2]) -> Self {
        let offset = LogicalScale::from_f32x2(value, screen_size);

        Self {
            x: self.x + offset.x * 2.0,
            y: self.y + offset.y * 2.0,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<[f32; 2]> for LogicalPosition {
    fn into(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl LogicalScale {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn from_u32x2(value: [u32; 2], screen_size: [u32; 2]) -> Self {
        Self::from_f32x2([value[0] as f32, value[1] as f32], screen_size)
    }

    pub fn from_f32x2(value: [f32; 2], screen_size: [u32; 2]) -> Self {
        // Scale value between 0 and 1
        let x = value[0] / screen_size[0] as f32;
        let y = value[1] / screen_size[1] as f32;

        Self { x, y }
    }

    pub fn add_u32x2(&self, value: [i32; 2], screen_size: [u32; 2]) -> Self {
        self.add_f32x2([value[0] as f32, value[1] as f32], screen_size)
    }

    pub fn add_f32x2(&self, value: [f32; 2], screen_size: [u32; 2]) -> Self {
        let offset = Self::from_f32x2(value, screen_size);

        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<[f32; 2]> for LogicalScale {
    fn into(self) -> [f32; 2] {
        [self.x, self.y]
    }
}
