use std::{collections::HashSet, sync::Arc, time::Instant};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
}

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    time: Instant,
    resolution: PhysicalSize<u32>,
    camera_pos: [f32; 4],
    camera_rot: [f32; 4],
    pressed_keys: HashSet<KeyCode>,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RaymarchUniforms {
    time: f32,
    _pad: [f32; 3],
    resolution: [f32; 4],
    camera_pos: [f32; 4],
    camera_rot: [f32; 4],
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu raymarching"))
                .unwrap(),
        );

        let state = pollster::block_on(State::new(Arc::clone(&window)));
        self.window = Some(window);
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.resize(size);
                    state.resolution = size;
                }
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.render();
                }

                if let Some(_window) = &self.window {}
            }

            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if let (PhysicalKey::Code(keycode), Some(state)) =
                    (key_event.physical_key, &mut self.state)
                {
                    match key_event.state {
                        ElementState::Pressed => match keycode {
                            KeyCode::KeyW
                            | KeyCode::KeyA
                            | KeyCode::KeyS
                            | KeyCode::KeyD
                            | KeyCode::Space
                            | KeyCode::ControlLeft
                            | KeyCode::ControlRight
                            | KeyCode::ArrowUp
                            | KeyCode::ArrowLeft
                            | KeyCode::ArrowDown
                            | KeyCode::ArrowRight
                            | KeyCode::ShiftLeft
                            | KeyCode::ShiftRight => {
                                state.pressed_keys.insert(keycode);
                            }
                            _ => {}
                        },
                        ElementState::Released => {
                            state.pressed_keys.remove(&keycode);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let (Some(state), Some(window)) = (&mut self.state, &self.window) {
            state.update();
            window.request_redraw();
        }
    }
}

impl State {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
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
                required_limits: wgpu::Limits::default(),
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

        let uniform_data = RaymarchUniforms::default();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
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
                mask: 1,
                alpha_to_coverage_enabled: false,
            },

            multiview_mask: None,
            cache: None,
        });

        let time = std::time::Instant::now();

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            bind_group,
            uniform_buffer,
            time,
            resolution: PhysicalSize {
                width: size.width,
                height: size.height,
            },
            camera_pos: [0.0, 0.0, -3.0, 0.0],
            camera_rot: Default::default(),
            pressed_keys: HashSet::new(),
        }
    }

    fn update(&mut self) {
        let time = Instant::now().duration_since(self.time).as_secs_f32();
        let resolution = [
            self.resolution.width as f32,
            self.resolution.height as f32,
            0.0,
            0.0,
        ];

        let yaw = self.camera_rot[0];
        let pitch = self.camera_rot[1];

        let forward = [
            pitch.cos() * yaw.sin(),
            -pitch.sin(),
            pitch.cos() * yaw.cos(),
        ];
        let right = [
            yaw.cos(),
            0.0,
            -yaw.sin(),
        ];

        let mut speed = 0.1;

        if self.pressed_keys.contains(&KeyCode::ShiftLeft) {
            speed *= 5.0;
        }

        for key in &self.pressed_keys {
            match key {
                KeyCode::KeyW => {
                    self.camera_pos[0] += forward[0] * speed;
                    self.camera_pos[1] += forward[1] * speed;
                    self.camera_pos[2] += forward[2] * speed;
                }
                KeyCode::KeyS => {
                    self.camera_pos[0] -= forward[0] * speed;
                    self.camera_pos[1] -= forward[1] * speed;
                    self.camera_pos[2] -= forward[2] * speed;
                }
                KeyCode::KeyA => {
                    self.camera_pos[0] -= right[0] * speed;
                    self.camera_pos[1] -= right[1] * speed;
                    self.camera_pos[2] -= right[2] * speed;
                }
                KeyCode::KeyD => {
                    self.camera_pos[0] += right[0] * speed;
                    self.camera_pos[1] += right[1] * speed;
                    self.camera_pos[2] += right[2] * speed;
                }
                KeyCode::Space => self.camera_pos[1] += speed,
                KeyCode::ControlLeft | KeyCode::ControlRight => self.camera_pos[1] -= speed,

                KeyCode::ArrowUp => self.camera_rot[1] -= 0.01,
                KeyCode::ArrowLeft => self.camera_rot[0] -= 0.01,
                KeyCode::ArrowDown => self.camera_rot[1] += 0.01,
                KeyCode::ArrowRight => self.camera_rot[0] += 0.01,
                _ => {}
            }
        }

        let uniform_data = RaymarchUniforms {
            time,
            resolution,
            camera_pos: self.camera_pos,
            camera_rot: self.camera_rot,
            ..Default::default()
        };

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));
    }

    pub fn render(&mut self) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),

                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],

                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
