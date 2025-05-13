use tracing::debug;

use crate::capture_taker::CaptureTaker;

use super::{
    ApplicationState, active::ActiveApplication, inactive::InactiveApplication,
    loading::LoadingApplication,
};

pub struct ExitedApplication;

impl ApplicationState for ExitedApplication {
    fn handle_event(self: Box<Self>, _event: super::ApplicationEvent) -> Box<dyn ApplicationState> {
        self
    }
}

impl From<InactiveApplication> for ExitedApplication {
    fn from(value: InactiveApplication) -> Self {
        debug!("[TRANSITION] Inactive -> Exited");

        unsafe { value.core.vulkan.device_wait_idle() };

        Self {}
    }
}

impl From<LoadingApplication> for ExitedApplication {
    fn from(value: LoadingApplication) -> Self {
        debug!("[TRANSITION] Loading -> Exited");

        unsafe { value.core.vulkan.device_wait_idle() };

        if let Some(capture) = value.capture {
            value.core.capture_taker.cleanup_windows_capture(capture);
        }

        if let Some(capture) = value.hdr_capture {
            unsafe { capture.destroy(&value.core.vulkan) };
        }

        Self {}
    }
}

impl From<ActiveApplication> for ExitedApplication {
    fn from(value: ActiveApplication) -> Self {
        debug!("[TRANSITION] Active -> Exited");

        unsafe { value.core.vulkan.device_wait_idle() };

        value
            .core
            .capture_taker
            .cleanup_windows_capture(value.capture.capture);

        unsafe { value.capture.hdr_capture.destroy(&value.core.vulkan) };

        Self {}
    }
}
