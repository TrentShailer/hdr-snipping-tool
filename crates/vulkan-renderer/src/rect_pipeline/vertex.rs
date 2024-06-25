use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex as VertexDerive};

#[derive(BufferContents, VertexDerive)]
#[repr(C)]
pub struct Vertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],

    #[format(R8G8B8A8_UNORM)]
    pub color: [u8; 4],
}
