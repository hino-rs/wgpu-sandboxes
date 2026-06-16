use egui::Context as EguiContext;
use egui_wgpu::Renderer as EguiRenderer;
use egui_wgpu::RendererOptions;
use egui_winit::State as EguiState;
use std::{collections::HashSet, sync::Arc};
use web_time::Instant;
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
    egui_ctx: EguiContext,
    egui_state: Option<EguiState>,
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
    uniforms: Uniforms,
    // resolution: PhysicalSize<u32>,
    // camera_pos: [f32; 4],
    // camera_rot: [f32; 4],
    pressed_keys: HashSet<KeyCode>,
    // params: [f32; 4],
    egui_renderer: EguiRenderer,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
    max_dt: f32,
    min_dt: f32,
    _pad: f32,
    resolution: [f32; 4],
    camera_pos: [f32; 4],
    camera_rot: [f32; 4],
    // params: [f32; 4],
    t_max: f32,
    max_step: u32,
    density_coef: f32,
    exposure_coef: f32,
    absorption_coef: f32,
    _pad2: [f32; 3],
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu raymarching"))
                .unwrap(),
        );

        let state = pollster::block_on(State::new(Arc::clone(&window)));

        let egui_state = EguiState::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        let egui_ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "font_ja".to_owned(),
            Arc::from(egui::FontData::from_owned(
                include_bytes!("../../../common/NotoSansJP-VariableFont_wght.ttf").to_vec(),
            )),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "font_ja".to_owned());
        egui_ctx.set_fonts(fonts);

        self.window = Some(window);
        self.state = Some(state);
        self.egui_ctx = egui_ctx;
        self.egui_state = Some(egui_state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(egui_state) = &mut self.egui_state {
            let response = egui_state.on_window_event(self.window.as_ref().unwrap(), &event);
            if response.consumed {
                return;
            }
        }

        match event {
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.resize(size);
                    state.uniforms.resolution = [size.width as f32, size.height as f32, 0.0, 0.0];
                }
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(egui_state), Some(window)) =
                    (&mut self.state, &mut self.egui_state, &self.window)
                {
                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
                        ui.label("最大刻み間隔");
                        ui.add(egui::Slider::new(&mut state.uniforms.max_dt, 0.01..=5.0));

                        ui.label("最小刻み間隔");
                        ui.add(egui::Slider::new(&mut state.uniforms.min_dt, 0.0001..=3.0));

                        ui.label("レイの最大移動距離");
                        ui.add(egui::Slider::new(&mut state.uniforms.t_max, 512.0..=2048.0));

                        ui.label("マーチングの最大ステップ数");
                        ui.add(egui::Slider::new(&mut state.uniforms.max_step, 32..=512));

                        ui.label("ガスの密度係数");
                        ui.add(egui::Slider::new(
                            &mut state.uniforms.density_coef,
                            0.00..=1.00,
                        ));

                        ui.label("露光係数");
                        ui.add(egui::Slider::new(
                            &mut state.uniforms.exposure_coef,
                            0.0..=100.0,
                        ));
                        
                        ui.label("ガスの光の吸収率係数");
                        ui.add(egui::Slider::new(
                            &mut state.uniforms.absorption_coef,
                            0.00..=30.00,
                        ));
                    });

                    let egui_output = self.egui_ctx.end_pass();
                    egui_state.handle_platform_output(window, egui_output.platform_output);

                    for (id, image_delta) in &egui_output.textures_delta.set {
                        state.egui_renderer.update_texture(
                            &state.device,
                            &state.queue,
                            *id,
                            image_delta,
                        );
                    }

                    for id in &egui_output.textures_delta.free {
                        state.egui_renderer.free_texture(id);
                    }

                    let paint_jobs = self
                        .egui_ctx
                        .tessellate(egui_output.shapes, egui_output.pixels_per_point);

                    let screen_descripter = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [state.config.width, state.config.height],
                        pixels_per_point: egui_output.pixels_per_point,
                    };

                    state.render(&paint_jobs, &screen_descripter);
                }
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
        let mut size = window.inner_size();
        #[cfg(target_arch = "wasm32")]
        {
            if size.width == 0 || size.height == 0 {
                let web_window = web_sys::window().unwrap();
                let dpr = web_window.device_pixel_ratio();
                let width = (web_window.inner_width().unwrap().as_f64().unwrap() * dpr) as u32;
                let height = (web_window.inner_height().unwrap().as_f64().unwrap() * dpr) as u32;
                size = winit::dpi::PhysicalSize::new(width.max(1), height.max(1));

                use winit::platform::web::WindowExtWebSys;
                if let Some(canvas) = window.canvas() {
                    canvas.set_width(size.width);
                    canvas.set_height(size.height);
                }
            }
        }

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
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

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

        let uniform_data = Uniforms::default();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let image_bytes = include_bytes!("../../../assets/milky_way.jpg");
        let img = image::load_from_memory(image_bytes).unwrap().to_rgba8();
        let (width, height) = img.dimensions();

        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let sky_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sky Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &sky_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            texture_size,
        );

        let sky_texture_view = sky_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat, // 横方向はループさせる
            address_mode_v: wgpu::AddressMode::ClampToEdge, // 縦方向は端で止める
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear, // 綺麗に補間する
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&sky_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sky_sampler),
                },
            ],
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

        let time = Instant::now();

        let egui_renderer = EguiRenderer::new(&device, config.format, RendererOptions::default());

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            bind_group,
            uniform_buffer,
            time,
            uniforms: Uniforms {
                time: 0.0,
                min_dt: 0.01,
                max_dt: 0.5,
                _pad: 0.0,
                resolution: [size.width as f32, size.height as f32, 0.0, 0.0],
                camera_pos: [0.0, 0.0, -50.0, 0.0],
                camera_rot: Default::default(),
                t_max: 1024.0,
                max_step: 256,
                density_coef: 0.5,
                exposure_coef: 10.0,
                absorption_coef: 0.5,
                _pad2: Default::default(),
            },
            pressed_keys: HashSet::new(),
            egui_renderer,
        }
    }

    fn update(&mut self) {
        let time = Instant::now().duration_since(self.time).as_secs_f32();
        self.uniforms.time = time;

        let yaw = self.uniforms.camera_rot[0];
        let pitch = self.uniforms.camera_rot[1];

        // 視線方向（前）ベクトルを計算 (単位ベクトル)
        let forward = [
            pitch.cos() * yaw.sin(),
            -pitch.sin(),
            pitch.cos() * yaw.cos(),
        ];
        // 右方向ベクトルを計算 (単位ベクトル)
        let right = [yaw.cos(), 0.0, -yaw.sin()];

        let mut speed = 0.1;

        if self.pressed_keys.contains(&KeyCode::ShiftLeft) {
            speed *= 10.0;
        }

        for key in &self.pressed_keys {
            match key {
                KeyCode::KeyW => {
                    self.uniforms.camera_pos[0] += forward[0] * speed;
                    self.uniforms.camera_pos[1] += forward[1] * speed;
                    self.uniforms.camera_pos[2] += forward[2] * speed;
                }
                KeyCode::KeyS => {
                    self.uniforms.camera_pos[0] -= forward[0] * speed;
                    self.uniforms.camera_pos[1] -= forward[1] * speed;
                    self.uniforms.camera_pos[2] -= forward[2] * speed;
                }
                KeyCode::KeyA => {
                    self.uniforms.camera_pos[0] -= right[0] * speed;
                    self.uniforms.camera_pos[1] -= right[1] * speed;
                    self.uniforms.camera_pos[2] -= right[2] * speed;
                }
                KeyCode::KeyD => {
                    self.uniforms.camera_pos[0] += right[0] * speed;
                    self.uniforms.camera_pos[1] += right[1] * speed;
                    self.uniforms.camera_pos[2] += right[2] * speed;
                }
                KeyCode::Space => self.uniforms.camera_pos[1] += speed,
                KeyCode::ControlLeft | KeyCode::ControlRight => {
                    self.uniforms.camera_pos[1] -= speed
                }

                KeyCode::ArrowUp => self.uniforms.camera_rot[1] -= 0.01,
                KeyCode::ArrowLeft => self.uniforms.camera_rot[0] -= 0.01,
                KeyCode::ArrowDown => self.uniforms.camera_rot[1] += 0.01,
                KeyCode::ArrowRight => self.uniforms.camera_rot[0] += 0.01,
                _ => {}
            }
        }

        let uniform_data = self.uniforms;

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));
    }

    pub fn render(
        &mut self,
        paint_jobs: &[egui::epaint::ClippedPrimitive],
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
    ) {
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

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            paint_jobs,
            screen_descriptor,
        );

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

        {
            let mut egui_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();

            self.egui_renderer
                .render(&mut egui_pass, paint_jobs, screen_descriptor);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

impl App {
    #[allow(unused)]
    pub fn with_precreated(window: Arc<Window>, state: State) -> Self {
        let egui_ctx = EguiContext::default();
        let egui_state = EguiState::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        Self {
            window: Some(window),
            state: Some(state),
            egui_ctx,
            egui_state: Some(egui_state),
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run() {
    use std::sync::Arc;
    use winit::window::Window;

    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    let event_loop = EventLoop::new().unwrap();

    wasm_bindgen_futures::spawn_local(async move {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu Blackhole Simulator"))
                .unwrap(),
        );

        {
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().unwrap();
            let web_window = web_sys::window().unwrap();
            
            // Set initial canvas size based on window size and device pixel ratio
            let dpr = web_window.device_pixel_ratio();
            let width = (web_window.inner_width().unwrap().as_f64().unwrap() * dpr) as u32;
            let height = (web_window.inner_height().unwrap().as_f64().unwrap() * dpr) as u32;
            canvas.set_width(width);
            canvas.set_height(height);

            let document = web_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(&canvas).unwrap();
        }

        let state = State::new(Arc::clone(&window)).await;

        let mut app = App::with_precreated(window, state);
        event_loop.run_app(&mut app).unwrap();
    });
}
