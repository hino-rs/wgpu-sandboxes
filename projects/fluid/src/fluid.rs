use crate::{app::MouseStateUniform, gpu::GpuContext};
use wgpu::{CommandEncoder, util::DeviceExt};

// =======================================================
// 定義
// =======================================================
pub const INITIAL_NUM_FLUID_PARTICLES: usize = 1500;

// 流体シミュレーター
pub struct FluidSim {
    pub pause: bool,
    pub delay: u64,
    pub next_step: bool,
    pub glow_width: f32, // 粒子をぼかす幅
    pub cursor_radius: f32,
    pub num_particles: usize,
    pub params: ParticlesParams,
    pub params_buffer: wgpu::Buffer,
    pub mouse_buffer: wgpu::Buffer,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,

    // Compute
    pub compute_pipeline: wgpu::ComputePipeline,
    pub particles_buffers: ParticlesBuffers,
    pub compute_bind_group_a: wgpu::BindGroup,
    pub compute_bind_group_b: wgpu::BindGroup,
}

// 流体粒子
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
}

// 流体粒子のパラメータ
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticlesParams {
    pub visual_range: f32,      // 他者への認識範囲
    pub protected_range: f32,   //
    pub separation_weight: f32, //
    pub alignment_weight: f32,  //
    pub cohesion_weight: f32,   //
    pub max_speed: f32,         // 最高速度
    pub min_speed: f32,         // 最低速度

    _p: f32,
}

// ダブルバッファリング用
pub struct ParticlesBuffers {
    pub buffer_a: wgpu::Buffer,
    pub buffer_b: wgpu::Buffer,
    pub frame_count: u32,
}

// =======================================================
// 実装
// =======================================================

// -------------------------------------------------------
// FluidSim
// -------------------------------------------------------
impl FluidSim {
    pub fn init(gpu: &GpuContext) -> Self {
        let device = &gpu.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
                    // Binding 0: 読み取り専用のSrcバッファ
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Binding 1: 書き込み可能なDstバッファ
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Binding 2: パラメータ用Uniformバッファ
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Binding 4: マウス操作用バッファ
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[Some(&compute_bind_group_layout)],
                immediate_size: 0,
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let initial_data = Self::generate_initial_particles();

        let buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particles Buffer A"),
            contents: bytemuck::cast_slice(&initial_data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
        });

        let buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particles Buffer B"),
            contents: bytemuck::cast_slice(&initial_data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
        });

        let particles_buffers = ParticlesBuffers {
            buffer_a,
            buffer_b,
            frame_count: 0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boids Params Buffer"),
            contents: bytemuck::cast_slice(&[ParticlesParams::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mouse_data = MouseStateUniform {
            pos_x: 0.0,
            pos_y: 0.0,
            radius: 0.0,
            button: 0,
        };
        
        let mouse_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Brush Buffer"),
            contents: bytemuck::cast_slice(&[mouse_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let compute_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group A"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particles_buffers.buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particles_buffers.buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: mouse_buffer.as_entire_binding(),
                }
            ],
        });

        // Src = B, Dst = A
        let compute_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group B"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particles_buffers.buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particles_buffers.buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: mouse_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            pause: false,
            delay: 0,
            next_step: false,
            glow_width: 0.1,
            cursor_radius: 0.15,
            num_particles: INITIAL_NUM_FLUID_PARTICLES,
            params: ParticlesParams::default(),
            params_buffer,
            mouse_buffer,
            compute_bind_group_layout,

            compute_pipeline,
            particles_buffers,
            compute_bind_group_a,
            compute_bind_group_b,
        }
    }

    fn generate_particles(num: usize) -> Vec<Particle> {
        let mut particles = Vec::with_capacity(num);

        for _ in 0..num {
            particles.push(Particle {
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

        particles
    }

    pub fn generate_initial_particles() -> Vec<Particle> {
        Self::generate_particles(INITIAL_NUM_FLUID_PARTICLES)
    }

    pub fn get_buffers(&self) -> (&wgpu::Buffer, &wgpu::Buffer) {
        self.particles_buffers.get_buffers()
    }

    pub fn update_params(&mut self, gpu: &GpuContext) {
        gpu.queue
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[self.params]))
    }

    pub fn change_num_particles(&mut self, gpu: &GpuContext) {
        let new_particles = Self::generate_particles(self.num_particles);

        let buffer_a = gpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Particles Buffer A"),
                contents: bytemuck::cast_slice(&new_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );
        let buffer_b = gpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Particles Buffer B"),
                contents: bytemuck::cast_slice(&new_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        let particles_buffers = ParticlesBuffers {
            buffer_a,
            buffer_b,
            frame_count: 0,
        };

        let compute_bind_group_a = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group A"),
            layout: &self.compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particles_buffers.buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particles_buffers.buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.mouse_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_bind_group_b = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group B"),
            layout: &self.compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particles_buffers.buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particles_buffers.buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.mouse_buffer.as_entire_binding(),
                },
            ],
        });

        self.particles_buffers = particles_buffers;
        self.compute_bind_group_a = compute_bind_group_a;
        self.compute_bind_group_b = compute_bind_group_b;
    }

    pub fn update(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);

        // ダブルバッファリング
        let bind_group = if self.particles_buffers.frame_count.is_multiple_of(2) {
            &self.compute_bind_group_a
        } else {
            &self.compute_bind_group_b
        };

        compute_pass.set_bind_group(0, bind_group, &[]);

        let workgroup_count = (self.num_particles + 63).div_ceil(64);
        compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);

        self.particles_buffers.frame_count += 1;
    }
}

// -------------------------------------------------------
// Particle
// -------------------------------------------------------
impl Particle {
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

// -------------------------------------------------------
// ParticlesParams
// -------------------------------------------------------
impl Default for ParticlesParams {
    fn default() -> Self {
        Self {
            visual_range: 0.15,
            protected_range: 0.8,
            separation_weight: 3.0,
            alignment_weight: 1.5,
            cohesion_weight: 3.0,
            max_speed: 0.02,
            min_speed: 0.0,

            _p: 0.0,
        }
    }
}

// -------------------------------------------------------
// ParticlesBuffers
// -------------------------------------------------------
impl ParticlesBuffers {
    pub fn get_buffers(&self) -> (&wgpu::Buffer, &wgpu::Buffer) {
        if self.frame_count.is_multiple_of(2) {
            (&self.buffer_a, &self.buffer_b)
        } else {
            (&self.buffer_b, &self.buffer_a)
        }
    }
}
