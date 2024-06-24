use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex as VertexDerive};

#[derive(BufferContents, VertexDerive)]
#[repr(C)]
pub struct Vertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
}

#[derive(BufferContents, VertexDerive)]
#[repr(C)]
pub struct InstanceData {
    #[format(R32G32_SFLOAT)]
    pub position_offset: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub scale: [f32; 2],

    #[format(R8_UINT)]
    pub texture_index: u8,

    #[format(R32G32_SFLOAT)]
    pub uv_offset: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub uv_scale: [f32; 2],

    #[format(R8G8B8A8_UNORM)]
    pub color: [u8; 4],
}
