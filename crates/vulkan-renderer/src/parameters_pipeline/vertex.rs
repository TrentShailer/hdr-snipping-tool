use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex as VertexDerive};

#[derive(Clone, Copy, BufferContents, VertexDerive)]
#[repr(C)]
pub struct Vertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
}

#[derive(Clone, Copy, BufferContents, VertexDerive)]
#[repr(C)]
pub struct InstanceData {
    #[format(R32G32_SFLOAT)]
    pub position_offset: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub size: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub bitmap_size: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub uv_offset: [f32; 2],
}
