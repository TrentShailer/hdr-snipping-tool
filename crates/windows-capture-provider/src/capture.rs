use crate::display::Display;

/// A capture and it's metadata.
pub struct Capture {
    /// The raw block of bytes that make up the capture, the data is in RGBA little endian f16.
    pub data: Box<[u8]>,

    /// The display the capture is of.
    pub display: Display,
}
