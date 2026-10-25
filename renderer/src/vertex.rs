#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub const FULL_QUAD: [Vertex; 6] = [
    Vertex {
        position: [-1.0, -1.0],
        tex_coord: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
        tex_coord: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coord: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0],
        tex_coord: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coord: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, 1.0],
        tex_coord: [0.0, 0.0],
    },
];
