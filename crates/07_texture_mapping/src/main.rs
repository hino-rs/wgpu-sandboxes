use std::sync::Arc;
use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop, window::{Window, WindowId}};
use wgpu::util::DeviceExt;
use winit::event_loop::EventLoop;

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0], tex_cords: [0.0, 1.0] },
    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0], tex_cords: [1.0, 1.0] },
    Vertex { position: [0.5, 0.5, 0.0], color: [1.0, 0.0, 0.0], tex_cords: [1.0, 0.0] },
    Vertex { position: [-0.5, 0.5, 0.0], color: [1.0, 0.0, 0.0], tex_cords: [0.0, 0.0] },
];

const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 0
];

const WIDTH: usize = 16;
const HEIGHT: usize = 16;

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    tex_cords: [f32; 2],
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,

    index_buffer: wgpu::Buffer,
    num_indices: u32,
    diffuse_bind_group: wgpu::BindGroup,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu triangle"))
                .unwrap()
        );

        let state = pollster::block_on(State::new(Arc::clone(&window)));
        self.window = Some(window);
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
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

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
        ];

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
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
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vetex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let num_vertices = VERTICES.len() as u32;

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        // --------------------------------------------------------------------
        // 1. テクスチャ用画像データの生成 (CPUメモリ上)
        // --------------------------------------------------------------------
        // RGBA (1ピクセル4バイト) のピクセルデータを格納するバッファを作成します。
        let mut pixels = Vec::with_capacity(WIDTH * HEIGHT * 4);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                // xとyの座標から、2x2ピクセル単位の市松模様 (チェッカーボード) を作ります
                let is_white = ((x / 2) + (y / 2)) % 2 == 0;

                if is_white {
                    // 白 (RGBA: 255, 255, 255, 255)
                    pixels.extend_from_slice(&[255, 255, 255, 255]);
                } else {
                    // 黒 (RGBA: 0, 0, 0, 255)
                    pixels.extend_from_slice(&[0, 0, 0, 255]);
                }
            }
        }

        // --------------------------------------------------------------------
        // 2. GPUテクスチャの作成 (wgpu::Texture)
        // --------------------------------------------------------------------
        let texture_size = wgpu::Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1, // 2Dテクスチャなので奥行きは1
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Checkerboard Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // sRGBフォーマットを使用し、シェーダーでサンプリングした際に
            // 自動的に線形空間 (Linear) に変換されるようにします
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // COPY_DST: queue.write_textureでデータを書き込むために必要
            // TEXTURE_BINDING: シェーダー内から読み込むために必要
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // --------------------------------------------------------------------
        // 3. ピクセルデータをGPUテクスチャへ書き込む (queue.write_texture)
        // --------------------------------------------------------------------
        queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                // RGBA画像なので、1行あたりのバイト数は 4 * 幅 になります
                bytes_per_row: Some(4 * WIDTH as u32),
                rows_per_image: Some(HEIGHT as u32),
            },
            texture_size,
        );

        // --------------------------------------------------------------------
        // 4. テクスチャビュー (TextureView) の作成
        // --------------------------------------------------------------------
        // シェーダーからはテクスチャに直接アクセスせず、このビューを介して参照します
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // --------------------------------------------------------------------
        // 5. サンプラー (Sampler) の作成
        // --------------------------------------------------------------------
        // テクスチャのピクセルを取り出すときのルール（補間方法）を定義します
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // UV座標が [0.0, 1.0] の範囲外になった場合の処理 (端のピクセル色を伸ばす)
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // 拡大・縮小の際、ドット絵風にくっきり見せるために Nearest フィルタを設定
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // --------------------------------------------------------------------
        // 6. バインドグループとレイアウト (BindGroup / BindGroupLayout) の作成
        // --------------------------------------------------------------------
        // シェーダーの binding: 0, 1 に渡すリソースのレイアウト（定義）
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
            entries: &[
                // binding 0: テクスチャビュー用
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // binding 1: サンプラー用
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }
            ]
        });

        // 定義に基づいて、実際のビューとサンプラーをバインドグループに登録
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&texture_bind_group_layout)],
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
                mask: 1,
                alpha_to_coverage_enabled: false,
            },

            multiview_mask: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            num_indices,
            index_buffer,
            diffuse_bind_group,
        }
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
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Validation => { return; }
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            // インデックスバッファをセットする
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // インデックスを使って描画
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

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
