use hdr_capture::HdrCapture;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use snafu::{ResultExt, Snafu};
use windows::{
    core::ComInterface,
    Graphics::Capture::Direct3D11CaptureFrame,
    Win32::{
        Graphics::Direct3D11::{
            ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
            D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC,
            D3D11_USAGE_STAGING,
        },
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    },
};
use winit::dpi::PhysicalSize;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Windows API call '{call}' returned an error."))]
    WindowsApi {
        source: windows::core::Error,
        call: &'static str,
    },
}

pub fn process_frame(
    frame: Direct3D11CaptureFrame,
    d3d_device: &ID3D11Device,
    d3d_context: &ID3D11DeviceContext,
) -> Result<HdrCapture, Error> {
    // Copy frame into new texture
    let texture = unsafe {
        let surface = frame.Surface().context(WindowsApiSnafu {
            call: "frame.Surface()",
        })?;

        let access: IDirect3DDxgiInterfaceAccess = surface.cast().context(WindowsApiSnafu {
            call: "surface.cast()",
        })?;

        let source_texture = access
            .GetInterface::<ID3D11Texture2D>()
            .context(WindowsApiSnafu {
                call: "GetInterface::<ID3D11Texture2D>()",
            })?;

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        source_texture.GetDesc(&mut desc);
        desc.BindFlags = 0;
        desc.MiscFlags = 0;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;

        let copy_texture = {
            let mut texture = None;
            d3d_device
                .CreateTexture2D(&desc, None, Some(&mut texture))
                .context(WindowsApiSnafu {
                    call: "d3d_device.CreateTexture2D",
                })?;
            texture.unwrap()
        };

        d3d_context.CopyResource(
            Some(&copy_texture.cast().context(WindowsApiSnafu {
                call: "copy_texture.cast()",
            })?),
            Some(&source_texture.cast().context(WindowsApiSnafu {
                call: "source_texture
                    .cast()",
            })?),
        );

        copy_texture
    };

    let hdr_capture = unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc as *mut _);

        let resource: ID3D11Resource = texture.cast().context(WindowsApiSnafu {
            call: "texture.cast()",
        })?;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        d3d_context
            .Map(
                Some(&resource.clone()),
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut mapped),
            )
            .context(WindowsApiSnafu {
                call: "d3d_context.Map",
            })?;

        let slice: &[u8] = {
            std::slice::from_raw_parts(
                mapped.pData as *const _,
                (desc.Height * mapped.RowPitch) as usize,
            )
        };

        let data_width = mapped.RowPitch / 4 / 2;
        let expected_width = desc.Width;

        if data_width != expected_width {
            log::warn!(
                "data width '{}' does not match expected width '{}'",
                data_width,
                expected_width
            );
        }

        let capture = (0..slice.len() / 2)
            .into_par_iter()
            .map(|byte_index| {
                let start = byte_index * 2;
                f32_from_le_f16_bytes(slice[start], slice[start + 1])
            })
            .collect::<Box<[f32]>>();

        let size = PhysicalSize::new(mapped.RowPitch / 4 / 2, desc.Height);

        let hdr_capture = HdrCapture::new(capture, size);

        d3d_context.Unmap(Some(&resource), 0);

        hdr_capture
    };

    Ok(hdr_capture)
}

fn f32_from_le_f16_bytes(byte_0: u8, byte_1: u8) -> f32 {
    let sign: u8 = byte_1 & 0b1000_0000;
    let exponent: u8 = ((byte_1 & 0b0111_1100) >> 2) + 0b01110000;
    let fration_l: u8 = (byte_1 & 0b0000_0011) << 5 | byte_0 >> 3;
    let fration_r: u8 = byte_0 << 5;

    f32::from_le_bytes([
        0,
        fration_r,
        exponent << 7 | fration_l,
        sign | exponent >> 1,
    ])
}
