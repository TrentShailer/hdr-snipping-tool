use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex as VertexDerive};

#[derive(BufferContents, VertexDerive)]
#[repr(C)]
pub struct Vertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
}

#[derive(BufferContents, VertexDerive)]
#[repr(C)]
pub struct InstanceData {
    #[format(R32G32_SFLOAT)]
    position_offset: [f32; 2],
    #[format(R32_SFLOAT)]
    scale: f32,
}
