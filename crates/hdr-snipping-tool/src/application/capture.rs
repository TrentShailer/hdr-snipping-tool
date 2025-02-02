use std::sync::Arc;

use ash_helper::VulkanContext;
use tracing::error;
use vulkan::{HdrImage, Vulkan};
use windows_capture_provider::{Monitor, SendHWND, WindowsCapture};

use crate::{selection::Selection, utilities::windows_helpers::get_foreground_window};

#[allow(unused)]
pub struct Capture {
    pub vulkan: Arc<Vulkan>,
    pub monitor: Monitor,
    pub formerly_focused_window: SendHWND,
    pub whitepoint: f32,
    pub selection: Selection,
    pub windows_capture: Option<WindowsCapture>,
    pub hdr_capture: Option<HdrImage>,
}

impl Capture {
    pub fn new(vulkan: Arc<Vulkan>, monitor: Monitor) -> Self {
        let size = monitor.size();

        Self {
            vulkan,
            monitor,
            formerly_focused_window: SendHWND(get_foreground_window()),
            whitepoint: monitor.sdr_white,
            selection: Selection::new([0.0, 0.0].into(), [size[0] as f32, size[1] as f32].into()),
            windows_capture: None,
            hdr_capture: None,
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        unsafe {
            let queue = self.vulkan.queue(vulkan::QueuePurpose::Graphics).lock();
            if let Err(e) = self.vulkan.device().queue_wait_idle(*queue) {
                error!("Could not wait for queue idle:\n{e}");
            };
            drop(queue);

            if let Some(capture) = self.hdr_capture {
                capture.destroy(&self.vulkan);
            }
        }

        // TODO destroy windows capture handle, requires DX
    }
}
