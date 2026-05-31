use std::sync::Arc;

use egui;
use egui_wgpu::RendererOptions;
use wgpu::util::DeviceExt;
use winit::window::Window;
use egui_wgpu::Renderer as EguiRenderer;

use crate::object::{Board, INITIAL_NUM_GRID_PER_ROW, InstanceRaw, SQUARE, Square, TERRAIN_COLOR, Vertex, WATER_DEEP, WATER_SHALLOW};

pub struct SimBuffers {
    pub buffer_a: wgpu::Buffer,
    pub buffer_b: wgpu::Buffer,
    pub frame_count: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BrushUniform {
    pub pos_x: f32,
    pub pos_y: f32,
    pub radius: f32,
    pub strength: f32,
    pub is_active: u32,
    pub _pad: [u32; 3], // 16-byte alignment padding
}

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
    
    // compute shader用
    compute_pipeline: wgpu::ComputePipeline,
    pub sim_buffers: SimBuffers,
    pub compute_bind_group_a: wgpu::BindGroup,
    pub compute_bind_group_b: wgpu::BindGroup,
    
    // brush用
    brush_buffer: wgpu::Buffer,
    pub brush_data: BrushUniform,
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
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            }
        );

        let egui_renderer = EguiRenderer::new(
            &device,
            config.format,
            RendererOptions::default(),
        );

        // ----------------------------------------
        // Compute Shaderのため
        // ----------------------------------------
        let initial_data = Board::empty_board(INITIAL_NUM_GRID_PER_ROW * INITIAL_NUM_GRID_PER_ROW);

        let buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boids Buffer A"),
            contents: bytemuck::cast_slice(&initial_data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
        });
        let buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boids Buffer B"),
            contents: bytemuck::cast_slice(&initial_data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
        });

        let sim_buffers = SimBuffers {
            buffer_a,
            buffer_b,
            frame_count: 0,
        };

        let brush_data = BrushUniform {
            pos_x: 0.0,
            pos_y: 0.0,
            radius: 0.0,
            strength: 0.0,
            is_active: 0,
            _pad: [0; 3],
        };

        let brush_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Brush Buffer"),
            contents: bytemuck::cast_slice(&[brush_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
                    // Binding 0: 読み取り専
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
                    // Binding 1: 書き込み用
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
                    // Binding 2: レンダリング用
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Binding 3: ブラシUniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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

        let compute_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group A"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_buffers.buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_buffers.buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry { 
                    binding: 2, 
                    resource: instance_buffer.as_entire_binding() 
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: brush_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group B"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_buffers.buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_buffers.buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry { 
                    binding: 2, 
                    resource: instance_buffer.as_entire_binding() 
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: brush_buffer.as_entire_binding(),
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
            egui_renderer,
            compute_pipeline,
            sim_buffers,
            compute_bind_group_a,
            compute_bind_group_b,
            brush_buffer,
            brush_data,
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
        
        let cell_pitch = 2.0 / (num_grid_per_row - 1) as f32;
        let cell_scale = cell_pitch;
        
        for y in 0..num_grid_per_row {
            for x in 0..num_grid_per_row {
                let x_pos = (x as f32) * cell_pitch - 1.0;
                let y_pos = (y as f32) * cell_pitch - 1.0;

                let square = squares[y * num_grid_per_row + x];
                
                let color = if square.puddle <= 0.005 {
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

    pub fn update_compute(&mut self, num_instances: usize) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            
            // ダブルバッファリングのバインドグループ切り替え
            let bind_group = if self.sim_buffers.frame_count % 2 == 0 {
                &self.compute_bind_group_a
            } else {
                &self.compute_bind_group_b
            };
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            // ワークグループの実行 (サイズ 64)
            let workgroup_count = (num_instances + 63) / 64;
            compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        self.sim_buffers.frame_count += 1;
    }

    // マウスクリック等でCPU側が更新されたとき、GPU側のバッファにデータを上書き同期する
    pub fn upload_board(&mut self, squares: &[Square]) {
        // 現在のアクティブな入力側バッファに書き込む
        let active_buffer = if self.sim_buffers.frame_count % 2 == 0 {
            &self.sim_buffers.buffer_a
        } else {
            &self.sim_buffers.buffer_b
        };
        
        self.queue.write_buffer(
            active_buffer,
            0,
            bytemuck::cast_slice(squares),
        );
    }

    pub fn update_brush(&mut self, active: bool, gx: f32, gy: f32, radius: f32, strength: f32) {
        self.brush_data.is_active = if active { 1 } else { 0 };
        self.brush_data.pos_x = gx;
        self.brush_data.pos_y = gy;
        self.brush_data.radius = radius;
        self.brush_data.strength = strength;

        self.queue.write_buffer(
            &self.brush_buffer,
            0,
            bytemuck::cast_slice(&[self.brush_data]),
        );
    }

    pub fn download_board(&mut self, squares: &mut [Square]) {
        let size = (squares.len() * std::mem::size_of::<Square>()) as wgpu::BufferAddress;
        
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer for Download"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Download Encoder"),
        });

        // 現在アクティブなシミュレーションバッファをコピー元にする
        let active_buffer = if self.sim_buffers.frame_count % 2 == 0 {
            &self.sim_buffers.buffer_a
        } else {
            &self.sim_buffers.buffer_b
        };

        encoder.copy_buffer_to_buffer(active_buffer, 0, &staging_buffer, 0, size);
        self.queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

        if let Ok(Ok(())) = receiver.recv() {
            let data = buffer_slice.get_mapped_range();
            let downloaded: &[Square] = bytemuck::cast_slice(&data);
            squares.copy_from_slice(downloaded);
            drop(data);
            staging_buffer.unmap();
        }
    }
}
