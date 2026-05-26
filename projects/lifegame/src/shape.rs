pub const GAP: f32 = 0.0;
pub const INITIAL_NUM_GRID_PER_ROW: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
        ];

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub scale: f32,
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }

    pub fn pure_instances(size: usize) -> Vec<Self> {
        vec![Self {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0],
            scale: 1.0,
        }; size]
    }
}

pub struct Shape;

impl Shape {
    pub const SQUARE: &[Vertex] = &[
        // 三角形1（左下半分）
        Vertex {
            position: [-0.5, -0.5, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        // 三角形2（右上半分）
        Vertex {
            position: [0.5, -0.5, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
            color: [1.0, 1.0, 1.0],
        },
    ];
}
