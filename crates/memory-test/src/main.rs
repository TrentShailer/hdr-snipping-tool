mod app;

use app::App;
use simplelog::{Config, TermLogger};
use winit::{event_loop::EventLoop, platform::run_on_demand::EventLoopExtRunOnDemand};

fn main() {
    TermLogger::init(
        log::LevelFilter::Info,
        Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Never,
    )
    .unwrap();

    let mut event_loop = EventLoop::<()>::with_user_event().build().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    //
    let mut app = App::new();

    // run the app
    event_loop.run_app_on_demand(&mut app).unwrap();
}
