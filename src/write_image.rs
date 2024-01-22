use windows::{
    core::{Result, HSTRING},
    Graphics::Imaging::{BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat},
    Storage::{CreationCollisionOption, FileAccessMode, StorageFolder},
};

pub fn write_image(image: Vec<u8>, width: u32, height: u32) -> Result<()> {
    let path = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let folder = StorageFolder::GetFolderFromPathAsync(&HSTRING::from(&path))?.get()?;
    let file = folder
        .CreateFileAsync(
            &HSTRING::from("screenshot.png"),
            CreationCollisionOption::ReplaceExisting,
        )?
        .get()?;

    {
        let stream = file.OpenAsync(FileAccessMode::ReadWrite)?.get()?;
        let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, &stream)?.get()?;
        encoder.SetPixelData(
            BitmapPixelFormat::Rgba8,
            BitmapAlphaMode::Premultiplied,
            width,
            height,
            1.0,
            1.0,
            &image,
        )?;

        encoder.FlushAsync()?.get()?;
    }

    Ok(())
}
