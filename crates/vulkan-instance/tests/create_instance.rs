use std::sync::mpsc::channel;

use test_helper::{get_window::get_window, logger::init_logger};
use vulkan_instance::VulkanInstance;

#[test]
fn create_instance() {
    init_logger();

    let (close_sender, close_receiver) = channel();
    get_window(
        |window| {
            let _vk = VulkanInstance::new(window, true).unwrap();
            close_sender.send(()).unwrap()
        },
        close_receiver,
    );
}
