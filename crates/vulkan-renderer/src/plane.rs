use crate::vertex::Vertex;

pub const PLANE_VERTICIES: [Vertex; 4] = [
    Vertex {
        position: [-1.0, -1.0],
        uv: [0.0, 0.0],
    }, // TL
    Vertex {
        position: [1.0, -1.0],
        uv: [1.0, 0.0],
    }, // TR
    Vertex {
        position: [1.0, 1.0],
        uv: [1.0, 1.0],
    }, // BR
    Vertex {
        position: [-1.0, 1.0],
        uv: [0.0, 1.0],
    }, // BL
];

pub const PLANE_INDICIES: [u32; 6] = [0, 1, 2, 2, 3, 0];
