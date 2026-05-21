use std::sync::Arc;
use winit::window::Window;

// GPU描画に必要な状態をまとめた構造体
pub struct State {
    // ウィンドウへの描画ターゲット
    surface: wgpu::Surface<'static>,
    // 論理GPUデバイス
    device: wgpu::Device,
    // GPUへのコマンド送信キュー
    queue: wgpu::Queue,
    // Surfaceのフォーマット・サイズ・Vsyncなどの設定
    config: wgpu::SurfaceConfiguration,
    // 頂点シェーダー
    render_pipeline: wgpu::RenderPipeline,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // ─────────────────────────────────────────────────────────────
        // Instance: Vulkan/Metal/DX12などのバックエンドを管理する。
        // ─────────────────────────────────────────────────────────────
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            // アプリケーションがどのバックエンドを使用するかを指定するビットフラグ
            backends: wgpu::Backends::all(), // 環境にあったバックエンドを自動選択
            // wgpuの内部動作の挙動を制御するフラグ
            flags: wgpu::InstanceFlags::default(), // DEBUG,VALIDATION,DISCARD_HAL_LABELSなど
            // 特定のバックエンドにのみ適用される個別のアドバンスド設定
            backend_options: wgpu::BackendOptions::default(),
            // OSのウィンドウシステムとの接続を確立するための、ディスプレイサーバーの生アドレッシング・ハンドルを保持する
            display: None,
            // VRAM不足を検知・回避するための閾値を設定
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });

        // ─────────────────────────────────────────────────────────────
        // Surface: ウィンドウと紐づいた描画先サーフェイス
        // ─────────────────────────────────────────────────────────────
        let surface = instance.create_surface(window).unwrap();

        // ─────────────────────────────────────────────────────────────
        // Adapter: 物理GPUの抽象。どのGPUを優先して使うかをここで選ぶ
        // ─────────────────────────────────────────────────────────────
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // 電源優先設定
                power_preference: wgpu::PowerPreference::default(), // OSに任せる
                // 描画ターゲットと互換性のあるアダプターを要求
                compatible_surface: Some(&surface),
                // フォールバックアダプターを強制使用
                force_fallback_adapter: false, // 通常のハードウェアGPUを要求する
            })
            .await
            .unwrap();

        // ─────────────────────────────────────────────────────────────
        // Device / Queue:
        // Device: GPUリソースの作成窓口
        // Queue:  GPUへの描画コマンドを送るキュー
        // ─────────────────────────────────────────────────────────────
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                // デバッグ時にこのデバイスを識別するためのデバッグラベル
                label: None,
                // アプリケーションが動作するために必須とするGPUの拡張機能のセットを指定
                required_features: wgpu::Features::empty(),
                // アプリケーションが要求するリソースの限界値の最小保証ラインを設定する
                // 限界値がアダプターの持つスペックを超えている場合、デバイスの生成に失敗する
                required_limits: wgpu::Limits::default(),
                // wgpuに対して、アプリケーションのメモリ使用傾向やパフォーマンスの優先度を伝えるヒントを指定
                memory_hints: Default::default(),
                // デバイスが実行したすべてのコマンドをファイルに記録するトレース機能の設定
                trace: wgpu::Trace::Off,
                // まだ標準化されていない、あるいは特定の環境向けに試験実装されている実験的な機能の有効化フラグ
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .unwrap();

        // ─────────────────────────────────────────────────────────────
        // Surface
        // ─────────────────────────────────────────────────────────────
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

        // ─────────────────────────────────────────────────────────────
        // シェーダー読み込み
        // ─────────────────────────────────────────────────────────────
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // ─────────────────────────────────────────────────────────────
        // パイプラインレイアウト
        // ─────────────────────────────────────────────────────────────
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        // ─────────────────────────────────────────────────────────────
        // レンダーパイプライン
        // ─────────────────────────────────────────────────────────────
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

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
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
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
