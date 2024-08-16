use std::{
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
};

use winit::{platform::run_on_demand::EventLoopExtRunOnDemand, window::Window};

use crate::winit::{create_event_loop, create_window, App};

pub fn get_window<F: Fn(Arc<Window>)>(callback: F, close_receiver: Receiver<()>) {
    let mut app = App {
        resumed_callback: |_event_loop, window| callback(window),

        window_event_callback: |event_loop, _, _| {
            match close_receiver.try_recv() {
                Ok(_) => event_loop.exit(),
                Err(e) => match e {
                    std::sync::mpsc::TryRecvError::Empty => return,
                    std::sync::mpsc::TryRecvError::Disconnected => panic!("{e}"),
                },
            };
        },

        window: None,
    };
    let mut event_loop = create_event_loop();
    event_loop.run_app_on_demand(&mut app).unwrap();
}
