use std::{sync::Arc, time::Instant};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
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

    // コンピュートパイプライン関連のリソース
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    // テクスチャAから読み込み、テクスチャBへ書き込むバインドグループ
    compute_bind_group_a_to_b: wgpu::BindGroup,
    // テクスチャBから読み込み、テクスチャAへ書き込むバインドグループ
    compute_bind_group_b_to_a: wgpu::BindGroup,

    // レンダーパイプライン関連のリソース
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group_layout: wgpu::BindGroupLayout,
    // 最新となったテクスチャAを描画するためのバインドグループ
    render_bind_group_a: wgpu::BindGroup,
    // 最新となったテクスチャBを描画するためのバインドグループ
    render_bind_group_b: wgpu::BindGroup,

    // 交互に切り替えて使用する2枚の物理テクスチャとそれらのビュー
    storage_texture_a: wgpu::Texture,
    storage_texture_view_a: wgpu::TextureView,
    storage_texture_b: wgpu::Texture,
    storage_texture_view_b: wgpu::TextureView,

    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,

    time: Instant,
    resolution: PhysicalSize<u32>,
    
    // 現在何フレーム目かを追跡し、ピンポンバッファの入れ替えに使用します
    frame_count: u32,
}

// シェーダと受け渡すユニフォーム構造体 (16バイト境界アライメント)
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderUniforms {
    time: f32,
    _pad: f32,
    resolution: [f32; 2],
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu Storage Texture Trail"))
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
                }
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.update();
                    state.render();
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

// 起動時およびウィンドウリサイズ時に、2枚のストレージテクスチャと各種バインドグループを作成・更新するヘルパー関数
fn create_storage_resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    sampler: &wgpu::Sampler,
    uniform_buffer: &wgpu::Buffer,
    compute_layout: &wgpu::BindGroupLayout,
    render_layout: &wgpu::BindGroupLayout,
) -> (
    wgpu::Texture,
    wgpu::TextureView,
    wgpu::Texture,
    wgpu::TextureView,
    wgpu::BindGroup,
    wgpu::BindGroup,
    wgpu::BindGroup,
    wgpu::BindGroup,
) {
    let width = width.max(1);
    let height = height.max(1);

    // --- テクスチャ A の作成 ---
    // STORAGE_BINDING: コンピュートシェーダからの書き込み用
    // TEXTURE_BINDING: 前フレームとしての読み込み、および画面描画時のサンプリング用
    // RENDER_ATTACHMENT: 起動時/リサイズ時にレンダーパスで真っ黒にクリアするため
    let storage_texture_a = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Storage Texture A"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING 
            | wgpu::TextureUsages::TEXTURE_BINDING 
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view_a = storage_texture_a.create_view(&wgpu::TextureViewDescriptor::default());

    // --- テクスチャ B の作成 ---
    let storage_texture_b = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Storage Texture B"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING 
            | wgpu::TextureUsages::TEXTURE_BINDING 
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view_b = storage_texture_b.create_view(&wgpu::TextureViewDescriptor::default());

    // --- 初回のテクスチャクリア ---
    // 未クリアのテクスチャにはVRAMのゴミデータが入っている場合があり、そのまま前フレームの
    // 読み込み（textureLoad）を行うと砂嵐のようなノイズが残像として蓄積してしまいます。
    // そのため、空のレンダーパスを実行して両方のテクスチャを事前に真っ黒 (Color::BLACK) に初期化します。
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Texture Clear Encoder"),
    });
    {
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Pass A"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view_a,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    {
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Pass B"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view_b,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    queue.submit(std::iter::once(encoder.finish()));

    // --- コンピュート用バインドグループ 1 (Aから読み込み、Bへ書き込む) ---
    let compute_bind_group_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Compute Bind Group A to B"),
        layout: compute_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0, // last_frame (読み込み)
                resource: wgpu::BindingResource::TextureView(&view_a),
            },
            wgpu::BindGroupEntry {
                binding: 1, // current_frame (書き込み)
                resource: wgpu::BindingResource::TextureView(&view_b),
            },
            wgpu::BindGroupEntry {
                binding: 2, // uniforms
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    // --- コンピュート用バインドグループ 2 (Bから読み込み、Aへ書き込む) ---
    let compute_bind_group_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Compute Bind Group B to A"),
        layout: compute_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0, // last_frame (読み込み)
                resource: wgpu::BindingResource::TextureView(&view_b),
            },
            wgpu::BindGroupEntry {
                binding: 1, // current_frame (書き込み)
                resource: wgpu::BindingResource::TextureView(&view_a),
            },
            wgpu::BindGroupEntry {
                binding: 2, // uniforms
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    // --- レンダリング用バインドグループ A (Aをサンプリングして画面に描く用) ---
    let render_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Render Bind Group A"),
        layout: render_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0, // input_texture
                resource: wgpu::BindingResource::TextureView(&view_a),
            },
            wgpu::BindGroupEntry {
                binding: 1, // sampler
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2, // uniforms
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    // --- レンダリング用バインドグループ B (Bをサンプリングして画面に描く用) ---
    let render_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Render Bind Group B"),
        layout: render_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0, // input_texture
                resource: wgpu::BindingResource::TextureView(&view_b),
            },
            wgpu::BindGroupEntry {
                binding: 1, // sampler
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2, // uniforms
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    (
        storage_texture_a,
        view_a,
        storage_texture_b,
        view_b,
        compute_bind_group_a_to_b,
        compute_bind_group_b_to_a,
        render_bind_group_a,
        render_bind_group_b,
    )
}

impl State {
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
            label: Some("Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // 1. ユニフォームバッファの初期作成
        let uniform_data = ShaderUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 2. 描画サンプラーの作成
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // 3. コンピュート用バインドグループのレイアウト定義
        // Binding 0: 前フレーム読み込み用テクスチャ (Sampled)
        // Binding 1: 今フレーム書き込み用テクスチャ (Storage)
        // Binding 2: 時間・解像度情報 (Uniform)
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
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
                ],
            });

        // 4. レンダリング用バインドグループのレイアウト定義
        // Binding 0: 描画対象テクスチャ (Sampled)
        // Binding 1: サンプラー (Sampler)
        // Binding 2: ユニフォームバッファ (Uniform)
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // 5. テクスチャとバインドグループを実際に生成する
        let (
            storage_texture_a,
            storage_texture_view_a,
            storage_texture_b,
            storage_texture_view_b,
            compute_bind_group_a_to_b,
            compute_bind_group_b_to_a,
            render_bind_group_a,
            render_bind_group_b,
        ) = create_storage_resources(
            &device,
            &queue,
            size.width,
            size.height,
            &sampler,
            &uniform_buffer,
            &compute_bind_group_layout,
            &render_bind_group_layout,
        );

        // 6. パイプラインの作成
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

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&render_bind_group_layout)],
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

        Self {
            surface,
            device,
            queue,
            config,
            compute_pipeline,
            compute_bind_group_layout,
            compute_bind_group_a_to_b,
            compute_bind_group_b_to_a,
            render_pipeline,
            render_bind_group_layout,
            render_bind_group_a,
            render_bind_group_b,
            storage_texture_a,
            storage_texture_view_a,
            storage_texture_b,
            storage_texture_view_b,
            sampler,
            uniform_buffer,
            time,
            resolution: PhysicalSize {
                width: size.width,
                height: size.height,
            },
            frame_count: 0,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // リサイズされたらテクスチャとバインドグループを再生成
            let (
                tex_a,
                view_a,
                tex_b,
                view_b,
                comp_bg_a_to_b,
                comp_bg_b_to_a,
                render_bg_a,
                render_bg_b,
            ) = create_storage_resources(
                &self.device,
                &self.queue,
                new_size.width,
                new_size.height,
                &self.sampler,
                &self.uniform_buffer,
                &self.compute_bind_group_layout,
                &self.render_bind_group_layout,
            );

            self.storage_texture_a = tex_a;
            self.storage_texture_view_a = view_a;
            self.storage_texture_b = tex_b;
            self.storage_texture_view_b = view_b;
            self.compute_bind_group_a_to_b = comp_bg_a_to_b;
            self.compute_bind_group_b_to_a = comp_bg_b_to_a;
            self.render_bind_group_a = render_bg_a;
            self.render_bind_group_b = render_bg_b;
            self.resolution = new_size;
        }
    }

    fn update(&self) {
        let time = Instant::now().duration_since(self.time).as_secs_f32();
        let uniform_data = ShaderUniforms {
            time,
            resolution: [self.resolution.width as f32, self.resolution.height as f32],
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
                label: Some("Command Encoder"),
            });

        // 偶数フレームか奇数フレームかで、テクスチャA/Bの読み書き役割をスイッチします
        let (active_compute_bg, active_render_bg) = if self.frame_count % 2 == 0 {
            // Aからロードし、Bに保存する
            // 描画するのは、最新の書き込み先である B
            (&self.compute_bind_group_a_to_b, &self.render_bind_group_b)
        } else {
            // Bからロードし、Aに保存する
            // 描画するのは、最新の書き込み先である A
            (&self.compute_bind_group_b_to_a, &self.render_bind_group_a)
        };

        // 1. コンピュートパスの実行（残像計算 ＋ 円の追加）
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, active_compute_bg, &[]);
            
            // 各軸で16ピクセルごとに1ワークグループを割り当てて実行
            let workgroup_count_x = (self.resolution.width + 15) / 16;
            let workgroup_count_y = (self.resolution.height + 15) / 16;
            compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, 1);
        }

        // 2. レンダーパスの実行（最新のテクスチャを画面にフルスクリーンで表示）
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, active_render_bg, &[]);
            render_pass.draw(0..3, 0..1); // フルスクリーン三角形の描画
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        // フレームカウントをインクリメントし、次回バッファを切り替えます
        self.frame_count = self.frame_count.wrapping_add(1);
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
