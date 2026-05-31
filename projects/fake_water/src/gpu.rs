use std::sync::Arc;

use egui;
use egui_wgpu::RendererOptions;
use wgpu::util::DeviceExt;
use winit::window::Window;
use egui_wgpu::Renderer as EguiRenderer;

use crate::object::{INITIAL_NUM_GRID_PER_ROW, InstanceRaw, SQUARE, Square, TERRAIN_COLOR, Vertex, WATER_DEEP, WATER_SHALLOW};

pub struct State {
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    instance_buffer: wgpu::Buffer,
    pub num_instances: u32,
    pub egui_renderer: EguiRenderer,
}

impl State {
    pub fn update_instance_buffer(&mut self, new_size: usize) {
        self.instance_buffer = self.device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: (new_size * std::mem::size_of::<InstanceRaw>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );
    }

    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex::desc(),
                    InstanceRaw::desc(),
                ],
                compilation_options: Default::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },

            depth_stencil: None,


            multisample: wgpu::MultisampleState {
                count: 1,
                mask: 11,
                alpha_to_coverage_enabled: false,
            },

            multiview_mask: None,

            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(SQUARE),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let num_vertices = SQUARE.len() as u32;

        let num_instances = (INITIAL_NUM_GRID_PER_ROW * INITIAL_NUM_GRID_PER_ROW) as u32;

        let instances = InstanceRaw::pure_instances(num_instances as usize);

        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        // let mut current = board.empty_board();
        // for i in 0..current.len() {
        //     if rand::random_bool(0.25) {
        //         current[i] = Cell::Alive;
        //     }
        // }

        let egui_renderer = EguiRenderer::new(
            &device,
            config.format,
            RendererOptions::default(),
        );
        
        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            num_vertices,
            vertex_buffer,
            instance_buffer,
            num_instances,
            // board,
            // next: current.clone(),
            // current,
            egui_renderer,
        }
        // state.update_instances();
    }

    pub fn render(&mut self, paint_jobs: &[egui::epaint::ClippedPrimitive], screen_descriptor: &egui_wgpu::ScreenDescriptor, bg_color: [f32; 3]) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated |
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout |
            wgpu::CurrentSurfaceTexture::Occluded |
            wgpu::CurrentSurfaceTexture::Validation => { return; }
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.egui_renderer.update_buffers(
            &self.device, 
            &self.queue, 
            &mut encoder, 
            paint_jobs, 
            screen_descriptor,
        );

        let bg = bg_color;
        let clear_color = wgpu::Color {
            r: bg[0] as f64,
            g: bg[1] as f64,
            b: bg[2] as f64,
            a: 1.0,
        };

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],

                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..self.num_instances);
        }

        // eguiのレンダーパス
        {
            let mut egui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // 重ね
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None
            }).forget_lifetime();

            self.egui_renderer.render(&mut egui_pass, paint_jobs, screen_descriptor);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn update_instances(&mut self, squares: &[Square], num_grid_per_row: usize) {
        let mut instances = Vec::new();
        
        let cell_pitch = 1.6 / (num_grid_per_row - 1) as f32;
        let cell_scale = cell_pitch;
        
        for y in 0..num_grid_per_row {
            for x in 0..num_grid_per_row {
                let x_pos = (x as f32) * cell_pitch - 0.8;
                let y_pos = (y as f32) * cell_pitch - 0.8;

                let square = squares[y * num_grid_per_row + x];
                
                let color = if square.puddle <= 0.0 {
                    TERRAIN_COLOR
                } else {
                    let water_color = crate::utils::Math::mix(WATER_SHALLOW, WATER_DEEP, square.puddle);
                    let blended_rgb = crate::utils::Math::mix(TERRAIN_COLOR, water_color, water_color[3]);
                    
                    [
                        blended_rgb[0],
                        blended_rgb[1],
                        blended_rgb[2],
                        1.0,
                    ]
                };

                instances.push(InstanceRaw {
                    position: [x_pos, y_pos, 0.0],
                    color,
                    scale: cell_scale,
                });
            }
        }

        self.queue.write_buffer(
            &self.instance_buffer, 
            0, 
            bytemuck::cast_slice(&instances),
        );

        self.num_instances = instances.len() as u32;
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}
