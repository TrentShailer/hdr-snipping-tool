use tracing::debug;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

use crate::{
    application::core_resources::CoreResources, application_event_loop::Event,
    capture_taker::CaptureTaker, utilities::windows_helpers::set_foreground_window,
};

use super::{
    ApplicationEvent, ApplicationState, active::ActiveApplication, exited::ExitedApplication,
    loading::LoadingApplication,
};

pub struct InactiveApplication {
    pub core: CoreResources,
}

impl InactiveApplication {
    pub fn new(event_loop: &ActiveEventLoop, proxy: EventLoopProxy<Event>) -> Self {
        Self {
            core: CoreResources::new(event_loop, proxy),
        }
    }
}

impl ApplicationState for InactiveApplication {
    fn handle_event(self: Box<Self>, event: ApplicationEvent) -> Box<dyn ApplicationState> {
        match event {
            ApplicationEvent::ScreenshotKeyPressed => Box::new(LoadingApplication::from(*self)),

            ApplicationEvent::Shutdown => Box::new(ExitedApplication::from(*self)),

            _ => self,
        }
    }
}

impl From<LoadingApplication> for InactiveApplication {
    fn from(mut application: LoadingApplication) -> Self {
        debug!("[TRANSITION] Loading -> Inactive");

        let mut core = application.core;
        core.window.set_visible(false);
        set_foreground_window(application.previous_focused_window);

        // Clean up
        {
            core.renderer.set_hdr_capture(None);
            core.renderer.render();

            unsafe { core.vulkan.device_wait_idle() };

            if let Some(capture) = application.hdr_capture.take() {
                unsafe { capture.destroy(&core.vulkan) };
            }
            if let Some(capture) = application.capture.take() {
                core.capture_taker.cleanup_windows_capture(capture);
            }
        }

        Self { core }
    }
}

impl From<ActiveApplication> for InactiveApplication {
    fn from(application: ActiveApplication) -> Self {
        debug!("[TRANSITION] Active -> Inactive");

        let mut core = application.core;
        core.window.set_visible(false);
        set_foreground_window(application.previous_focused_window);

        {
            core.renderer.set_hdr_capture(None);
            core.renderer.render();

            unsafe { core.vulkan.device_wait_idle() };

            unsafe { application.capture.hdr_capture.destroy(&core.vulkan) };
            core.capture_taker
                .cleanup_windows_capture(application.capture.capture);
        }

        Self { core }
    }
}
