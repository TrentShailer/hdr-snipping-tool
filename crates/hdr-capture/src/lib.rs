mod capture_info;
mod capture_provider;
mod display_info;

pub use capture_info::CaptureInfo;
pub use capture_provider::CaptureProvider;
pub use display_info::DisplayInfo;

use winit::dpi::{PhysicalPosition, PhysicalSize};

pub fn is_point_inside<T: std::ops::Add<Output = T> + std::cmp::PartialOrd>(
    point: PhysicalPosition<T>,
    box_position: PhysicalPosition<T>,
    box_size: PhysicalSize<T>,
) -> bool {
    point.x >= box_position.x
        && point.y >= box_position.y
        && point.x <= box_position.x + box_size.width
        && point.y <= box_position.y + box_size.height
}
