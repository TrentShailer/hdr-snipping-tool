use tracing::debug;
use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
use windows_core::PCWSTR;

use crate::{DirectX, WinError};

use super::Monitor;

impl Monitor {
    /// Returns the currently active monitors.
    pub fn get_active_monitors(direct_x: &DirectX) -> Result<Vec<Self>, WinError> {
        // Get the output descriptors
        let dxgi_outputs = direct_x.dxgi_output_descriptors()?;

        let mut monitors = Vec::new();

        // For each output create the monitor if it is active.
        for descriptor in dxgi_outputs {
            let monitor = match Self::new(descriptor)? {
                Some(monitor) => monitor,
                None => {
                    let name =
                        unsafe { PCWSTR::from_raw(descriptor.DeviceName.as_ptr()).to_string() }
                            .unwrap_or("Invalid Name".to_string());

                    debug!(
                        "Inactive Monitor {{ handle: {:?}, name: \"{}\", rect: {:?} }}",
                        descriptor.Monitor, name, descriptor.DesktopCoordinates
                    );

                    continue;
                }
            };

            monitors.push(monitor);
        }

        Ok(monitors)
    }

    /// Returns the monitor that is currently hovered by the mouse.
    pub fn get_hovered_monitor(direct_x: &DirectX) -> Result<Option<Self>, WinError> {
        let monitors = Self::get_active_monitors(direct_x)?;

        for monitor in &monitors {
            debug!("Active {:?}", monitor);
        }

        let mut mouse_point = POINT::default();
        unsafe { GetCursorPos(&mut mouse_point) }.map_err(|e| WinError::new(e, "GetCursorPos"))?;
        debug!("Mouse Point: {:#?}", mouse_point);

        let monitor = monitors.into_iter().find(|monitor| {
            let left = monitor.desktop_coordinates.left;
            let right = monitor.desktop_coordinates.right;
            let top = monitor.desktop_coordinates.top;
            let bottom = monitor.desktop_coordinates.bottom;

            mouse_point.x >= left
                && mouse_point.x <= right
                && mouse_point.y >= top
                && mouse_point.y <= bottom
        });

        Ok(monitor)
    }
}
