mod display_info;
mod hdr_capture;
mod sdr_capture;
mod selection;

use std::{
    fs::File,
    io::{self, BufWriter},
    path::PathBuf,
};

use chrono::Local;
use image::{codecs::png::PngEncoder, GenericImageView, ImageBuffer, Rgba};
use snafu::{ResultExt, Snafu};

pub use display_info::DisplayInfo;
pub use hdr_capture::HdrCapture;
pub use sdr_capture::SdrCapture;
pub use selection::Selection;

use crate::Tonemapper;

pub struct Capture {
    pub hdr: HdrCapture,
    pub sdr: SdrCapture,
    pub selection: Selection,
    pub tone_mapper: Box<dyn Tonemapper>,
}

impl Capture {
    pub fn new(hdr: HdrCapture, tone_mapper: Box<dyn Tonemapper>) -> Self {
        let sdr_capture = SdrCapture::from_hdr(&hdr, tone_mapper.as_ref());
        let selection = Selection::from(&hdr);

        Self {
            hdr,
            sdr: sdr_capture,
            tone_mapper,
            selection,
        }
    }

    pub fn save_capture(&self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, SaveError> {
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(
            self.sdr.size.width,
            self.sdr.size.height,
            self.sdr.data.to_owned(),
        )
        .unwrap()
        .view(
            self.selection.pos.x,
            self.selection.pos.y,
            self.selection.size.width,
            self.selection.size.height,
        )
        .to_image();

        let name = format!("screenshot {}.png", Local::now().format("%F %H-%M-%S"));
        let path = PathBuf::from(name);

        let file = File::create(path).context(IoSnafu)?;
        let mut buffer = BufWriter::new(file);

        let encoder = PngEncoder::new(&mut buffer);

        img.write_with_encoder(encoder).context(ImageSnafu)?;

        Ok(img)
    }
}

#[derive(Debug, Snafu)]
pub enum SaveError {
    #[snafu(display("An IO error ocurred"))]
    Io { source: io::Error },
    #[snafu(display("An image error ocurred"))]
    Image { source: image::ImageError },
}
