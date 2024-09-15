use tracing::instrument;
use windows::Win32::Graphics::Dxgi::{IDXGIOutput6, DXGI_OUTPUT_DESC1};
use windows_core::Interface;
use windows_result::Result as WindowsResult;

use crate::DirectXDevices;

/// Gets the DXGI output descriptors for the current DXGI outputs.
#[instrument(skip_all, level = tracing::Level::DEBUG, err)]
pub fn get_output_descriptors(devices: &DirectXDevices) -> WindowsResult<Box<[DXGI_OUTPUT_DESC1]>> {
    let mut output_descs = vec![];
    let mut i = 0;

    while let Ok(output) = unsafe { devices.dxgi_adapter.EnumOutputs(i) } {
        let output_6: IDXGIOutput6 = output.cast()?;

        let output_desc_1 = unsafe { output_6.GetDesc1()? };

        output_descs.push(output_desc_1);

        i += 1;
    }

    Ok(output_descs.into_boxed_slice())
}
