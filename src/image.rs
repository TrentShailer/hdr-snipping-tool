use half::f16;

pub struct Image {
    pub rows: Vec<Vec<[f16; 4]>>,
    pub width: usize,
    pub height: usize,
    pub max_value: f16,
}

impl Image {
    pub fn new(width: usize, height: usize) -> Self {
        let rows = vec![vec![[f16::ZERO; 4]; width]; height];
        Self {
            rows,
            width,
            height,
            max_value: f16::ZERO,
        }
    }

    /// a > 0; 0 < γ < 1;<br>
    /// Maps from the domain \[0,a^(-1/γ)] to the domain \[0,1].<br>
    /// γ regulated contrast, lower = lower, but also increases exposure of underexposed parts if lower.<br>
    /// if a < 1 it can decrease the exposure of over exposed parts of the image.
    pub fn compress_gamma(&mut self, a: f32, γ: f32) {
        let mut max = f16::ZERO;
        for row in self.rows.iter_mut() {
            for pixel in row.iter_mut() {
                for channel in 0..3 {
                    Self::compress_gamma_value(&mut pixel[channel], a, γ);
                    if pixel[channel] > max {
                        max = pixel[channel];
                    }
                }
            }
        }
        self.max_value = max;
    }

    fn compress_gamma_value(channel: &mut f16, a: f32, γ: f32) {
        let f32_value = f16::to_f32(channel.to_owned());
        let new_value = a * f32_value.powf(γ);
        *channel = f16::from_f32(new_value);
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = vec![0u8; self.width * self.height * 4];
        for row in 0..self.height {
            for pixel in 0..self.width {
                for channel in 0..4 {
                    let byte_index = channel + (pixel * 4) + (row * self.width * 4);

                    let channel_value = self.rows[row][pixel][channel];
                    if channel == 3 {
                        bytes[byte_index] = channel_value.to_f32() as u8;
                    } else {
                        bytes[byte_index] =
                            Self::scale_0_1(channel_value, self.max_value).to_f32() as u8;
                    }
                }
            }
        }
        bytes
    }

    fn scale_0_1(input: f16, max_value: f16) -> f16 {
        let mut value = input / max_value;
        value *= f16::from_f32(255.0);
        value
    }
}
