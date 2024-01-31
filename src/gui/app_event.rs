use glium::Display;
use imgui::Context;
use imgui_glium_renderer::Renderer;
use imgui_winit_support::WinitPlatform;

#[derive(Debug)]
pub enum AppEvent {
    Show,
    Hide,
}

impl AppEvent {
    pub fn handle(
        &self,
        display: &Display,
        _imgui: &mut Context,
        _platform: &mut WinitPlatform,
        _renderer: &mut Renderer,
    ) {
        match self {
            AppEvent::Show => {
                display.gl_window().window().set_visible(true);
                display.gl_window().window().focus_window();
            }
            AppEvent::Hide => {
                display.gl_window().window().set_visible(false);
            }
        };
    }
}
