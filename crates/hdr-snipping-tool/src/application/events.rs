use vulkan::HdrImage;
use windows_capture_provider::{Monitor, WindowsCapture};
use winit::dpi::PhysicalPosition;

use crate::{application_event_loop::Event, capture_taker::Whitepoint};

pub enum ApplicationEvent {
    ScreenshotKeyPressed,
    MouseEvent(MouseEvent),
    KeyboardEvent(KeyboardEvent),
    LoadingEvent(LoadingEvent),
    WindowEvent(WindowEvent),
    Shutdown,
}

pub enum WindowEvent {
    RedrawRequested,
    Resized,
}

pub enum KeyboardEvent {
    EscapePressed,
    EnterPressed,
}

pub enum MouseEvent {
    Clicked(PhysicalPosition<f32>),
    Moved(PhysicalPosition<f32>),
    Released,
}

pub enum LoadingEvent {
    FoundMonitor(Monitor),
    GotCapture(WindowsCapture),
    ImportedCapture(HdrImage),
    SelectedWhitepoint(Whitepoint),
    Error,
}

mod from_impls {
    pub use super::*;

    impl From<ApplicationEvent> for Event {
        fn from(value: ApplicationEvent) -> Self {
            Self::ApplicationEvent(value)
        }
    }

    impl From<WindowEvent> for ApplicationEvent {
        fn from(value: WindowEvent) -> Self {
            Self::WindowEvent(value)
        }
    }
    impl From<WindowEvent> for Event {
        fn from(value: WindowEvent) -> Self {
            Self::from(ApplicationEvent::from(value))
        }
    }

    impl From<KeyboardEvent> for ApplicationEvent {
        fn from(value: KeyboardEvent) -> Self {
            Self::KeyboardEvent(value)
        }
    }
    impl From<KeyboardEvent> for Event {
        fn from(value: KeyboardEvent) -> Self {
            Self::from(ApplicationEvent::from(value))
        }
    }

    impl From<MouseEvent> for ApplicationEvent {
        fn from(value: MouseEvent) -> Self {
            Self::MouseEvent(value)
        }
    }
    impl From<MouseEvent> for Event {
        fn from(value: MouseEvent) -> Self {
            Self::from(ApplicationEvent::from(value))
        }
    }

    impl From<LoadingEvent> for ApplicationEvent {
        fn from(value: LoadingEvent) -> Self {
            Self::LoadingEvent(value)
        }
    }
    impl From<LoadingEvent> for Event {
        fn from(value: LoadingEvent) -> Self {
            Self::from(ApplicationEvent::from(value))
        }
    }
}
