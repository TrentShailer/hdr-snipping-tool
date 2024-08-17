use std::sync::mpsc::channel;

use scrgb_tonemapper::maximum::find_maximum;
use test_helper::{get_window::get_window, hdr_capture::get_hdr_image, logger::init_logger};
use vulkan_instance::VulkanInstance;

#[test]
fn maximum() {
    init_logger();

    let (close_sender, close_receiver) = channel();
    get_window(
        |window| {
            let vk = VulkanInstance::new(window, true).unwrap();

            maximum_inner(&vk);

            close_sender.send(()).unwrap()
        },
        close_receiver,
    );
}

fn maximum_inner(vk: &VulkanInstance) {
    let (hdr_image, hdr_memory, hdr_view, hdr_size) = get_hdr_image(vk);
    let maximum = find_maximum(vk, hdr_view, hdr_size).unwrap();
    assert_eq!(maximum, test_helper::hdr_capture::MAXIMUM);

    unsafe {
        vk.device.destroy_image_view(hdr_view, None);
        vk.device.destroy_image(hdr_image, None);
        vk.device.free_memory(hdr_memory, None);
    }
}
