use wgpu::util::DeviceExt;

use crate::gpu::State;

pub const INITIAL_NUM_BOIDS: usize = 1500;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoidsParams {
    pub visual_range: f32,
    pub protected_range: f32,
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    pub max_speed: f32,
    pub min_speed: f32,
    pub _padding: f32,
}

impl Default for BoidsParams {
    fn default() -> Self {
        Self {
            visual_range: 0.15,
            protected_range: 0.035,
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            max_speed: 0.03,
            min_speed: 0.01,
            _padding: 0.0,
        }
    }
}

pub struct Boids {
    pub pause: bool,
    pub delay: u64,
    pub next_tick: bool,
    pub params: BoidsParams,
    pub num_boids: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Boid {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
}

pub struct BoidsBuffers {
    pub buffer_a: wgpu::Buffer,
    pub buffer_b: wgpu::Buffer,
    pub frame_count: u32,
}

impl Boids {
    pub fn change_num_boids(&self, gpu: &mut State) {
        let new_boids = Self::generate_boids(self.num_boids);

        let buffer_a = gpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Boids Buffer A"),
                contents: bytemuck::cast_slice(&new_boids),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );
        let buffer_b = gpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Boids Buffer B"),
                contents: bytemuck::cast_slice(&new_boids),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        let boids_buffers = crate::boids::BoidsBuffers {
            buffer_a,
            buffer_b,
            frame_count: 0,
        };

        let compute_bind_group_a = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group A"),
            layout: &gpu.compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: boids_buffers.buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: boids_buffers.buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gpu.params_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_bind_group_b = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group B"),
            layout: &gpu.compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: boids_buffers.buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: boids_buffers.buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gpu.params_buffer.as_entire_binding(),
                },
            ],
        });

        gpu.boids_buffers = boids_buffers;
        gpu.compute_bind_group_a = compute_bind_group_a;
        gpu.compute_bind_group_b = compute_bind_group_b;
    }

    fn generate_boids(num: usize) -> Vec<Boid> {
        let mut boids = Vec::with_capacity(num);

        for _ in 0..num {
            boids.push(Boid {
                position: [
                    rand::random_range(-1.0..=1.0),
                    rand::random_range(-1.0..=1.0),
                ],
                velocity: [
                    rand::random_range(-0.1..=0.1),
                    rand::random_range(-0.1..=0.1),
                ],
            });
        }

        boids
    }

    pub fn generate_initial_boids() -> Vec<Boid> {
        Self::generate_boids(INITIAL_NUM_BOIDS)
    }
}

impl Boid {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl BoidsBuffers {
    pub fn get_buffers(&self) -> (&wgpu::Buffer, &wgpu::Buffer) {
        if self.frame_count % 2 == 0 {
            (&self.buffer_a, &self.buffer_b)
        } else {
            (&self.buffer_b, &self.buffer_a)
        }
    }
}
