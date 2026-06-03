use crate::{fluid::{FluidSim, INITIAL_NUM_FLUID_PARTICLES, Particle}, gpu::GpuContext, types::Rgb};
use wgpu::{CommandEncoder, TextureView, util::DeviceExt};
use winit::window::Window;

// wgpu描画
pub struct Renderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    bg_color: Rgb,
    render_bind_group: wgpu::BindGroup,
    render_params_buffer: wgpu::Buffer,
    render_params: RenderParams,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderParams {
    pub num_particles: u32,
    pub aspect_ratio: f32,
    pub glow_width: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x4,
        ];

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

impl Renderer {
    pub fn init(gpu: &GpuContext, window: &Window, particles_params_buffer: &wgpu::Buffer) -> Self {
        let device = &gpu.device;
        let config = &gpu.config;
        let size = window.inner_size();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },  
                ],
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&render_bind_group_layout)],
                immediate_size: 0,
            });

        let vertex = wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc(), Particle::desc()],
            compilation_options: Default::default(),
        };

        let fragment = wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        };

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        };

        let multisample = wgpu::MultisampleState {
            count: 1,
            mask: 11,
            alpha_to_coverage_enabled: false,
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex,
            fragment: Some(fragment),
            primitive,
            depth_stencil: None,
            multisample,
            multiview_mask: None,
            cache: None,
        });

        let initial_render_params = RenderParams {
            num_particles: INITIAL_NUM_FLUID_PARTICLES as u32,
            aspect_ratio: size.width as f32 / size.height as f32,
            glow_width: 1.0,
        };

        let render_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Render Params Buffer"),
            contents: bytemuck::cast_slice(&[initial_render_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
            wgpu::BindGroupEntry {
                binding: 2,
                resource: particles_params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: render_params_buffer.as_entire_binding(),
            }],
        });

        let circle_vertices = generate_circle_vertices(32);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&circle_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let bg_color = Rgb::BLACK;

        Self {
            render_pipeline,
            vertex_buffer,
            bg_color,
            render_bind_group,
            render_params_buffer,
            render_params: initial_render_params,
        }
    }

    pub fn draw_scene(
        &self, 
        encoder: &mut CommandEncoder, 
        view: &TextureView, 
        (src, _dst): (&wgpu::Buffer, &wgpu::Buffer), 
        num_particles: usize,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.bg_color.wgpu(1.0)),
                    store: wgpu::StoreOp::Store,
                },
            })],

            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, src.slice(..));
        render_pass.draw(0..(32 * 3) as u32, 0..num_particles as u32);        
    }

    pub fn update_render_params(&mut self, gpu: &GpuContext, fluid: &FluidSim, glow_width: f32) {
        let aspect_ratio = gpu.config.width as f32 / gpu.config.height as f32;
        
        let render_params = RenderParams {
            num_particles: fluid.num_particles as u32,
            aspect_ratio,
            glow_width,
        };
        
        gpu.queue.write_buffer(
            &self.render_params_buffer, 
            0, 
            bytemuck::cast_slice(&[render_params]),
        );

        self.render_params = render_params;
    }
}

fn generate_circle_vertices(segments: usize) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    for i in 0..segments {
        // 現在の角度と、隣の角度（ラジアン）を計算
        let theta1 = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
        let theta2 = ((i + 1) as f32 / segments as f32) * 2.0 * std::f32::consts::PI;

        // 1つの扇形（三角形）を構築して追加
        // 中心点
        vertices.push(Vertex {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        });
        // 円周上の点 1
        vertices.push(Vertex {
            position: [theta1.cos(), theta1.sin(), 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        });
        // 円周上の点 2
        vertices.push(Vertex {
            position: [theta2.cos(), theta2.sin(), 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    vertices
}

