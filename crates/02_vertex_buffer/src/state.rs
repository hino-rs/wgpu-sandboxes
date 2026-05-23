use std::sync::Arc;
use winit::window::Window;
use wgpu::util::DeviceExt;

// 頂点データの中身 (3点分)
const VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
];

// 頂点構造体
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    // アトリビュート (x, y, z)と(r, g, b)
    position: [f32; 3],
    color: [f32; 3], 
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3, // shader_location(0) に f32 x 3 (position)
            1 => Float32x3, // shader_location(1) に f32 x 3 (color)
        ];

        wgpu::VertexBufferLayout {
            // 1つの頂点データが何バイトか (GPUが次の頂点に進むための幅)
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            // 描画データが進むタイミング (頂点ごとに次のデータに進む)
            step_mode: wgpu::VertexStepMode::Vertex,
            // 頂点アトリビュート (位置座標と色)の構成
            attributes: ATTRIBUTES,
        }
    }
}

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
    // 頂点バッファの実態
    vertex_buffer: wgpu::Buffer,
    // 頂点の個数
    num_vertices: u32,
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
            // 画面(バックバッファ)のテクスチャをどのような目的・用途で使用するかを指定
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // 画面のピクセルデータの色表現フォーマットを指定
            format: surface_format,
            // 画面のテクスチャサイズを指定
            width: size.width,
            height: size.height,
            // 画面への描画更新タイミング(垂直同期:VSyncの挙動)を制御する
            present_mode: wgpu::PresentMode::Fifo,
            // ウィンドウの背景と、wgpuの描画内容をどのようにαブレンドするかを指定
            alpha_mode: surface_caps.alpha_modes[0],
            // 画面のテクスチャからテクスチャビューを作成する際に、元のフォーマットと異なるフォーマットして解釈することを許可するリスト
            view_formats: vec![],
            // GPUが処理を開始してから、実際に画面に表示されるまでにキューに溜めることができる最大フレーム数を指定
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
                // パイプラインにバインドする`BindGroupLayout`の配列を、バインドのグループのインデックス`@group(n)`順に並べたもの
                bind_group_layouts: &[],
                // プッシュ定数に相当するデータを格納するための即時利用メモリのサイズを指定
                immediate_size: 0,
            });

        // ─────────────────────────────────────────────────────────────
        // レンダーパイプライン
        // ─────────────────────────────────────────────────────────────
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            // シェーダーがどのバインドグループを使用するかの設計図を紐づける
            layout: Some(&render_pipeline_layout),

            // 頂点シェーダーの状態 
            // GPUがポリゴンの書くを計算するフェーズの設定。
            vertex: wgpu::VertexState {
                // コンパイル済みのシェーダーソースを指定
                module: &shader,
                // WGSLシェーダー内で、頂点処理の開始位置となる関数名を指定
                entry_point: Some("vs_main"),
                // 頂点バッファのメモリレイアウトを定義
                buffers: &[
                    Vertex::desc(), // 頂点バッファのレイアウトを登録
                ],
                // シェーダーコンパイル時の詳細オプション
                compilation_options: Default::default(),
            },

            // ピクセルシェーダーの状態
            // ポリゴンの内側を塗りつぶすフェーズの設定。
            fragment: Some(wgpu::FragmentState {
                // 頂点と同様
                module: &shader,
                entry_point: Some("fs_main"),
                // 描画先となるカラーバッファへの出力ルールを、ターゲットごとに配列で指定
                targets: &[Some(wgpu::ColorTargetState {
                    // 上の`SurfaceConfigration`で設定した画面の色形式と厳密に一致させる必要がある
                    format: config.format,
                    // 色の混色設定。`REPLACE`は完全に上書きする。
                    blend: Some(wgpu::BlendState::REPLACE),
                    // RGBAの度の成分に書き込みを許可するか。ALLはRGBA全て
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),

            // トポロジーとカリング
            // 頂点シェーダーが計算したバラバラの点を、どのように形として組み立てるかを指定する。
            primitive: wgpu::PrimitiveState {
                // 頂点をどう繋ぎ合わせるかを指定
                topology: wgpu::PrimitiveTopology::TriangleList,
                // topologyがStrip系の場合に、頂点インデックスの区切り方を指定するもの
                strip_index_format: None,
                // 三角形wの表をどちら向きにするかを決定する
                // Ccw: 画面上で頂点が半時計周りに並んでいる方を表と見なす (グラフィックス界の標準)
                // Cw: 時計回りを表と見なす
                front_face: wgpu::FrontFace::Ccw,
                // カメラに映らない「裏を向ているポリゴン」を描画スキップ(カリング)するかどうかを設定する
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },

            // 深度・ステンシル設定
            // 3D空間の前後関係を正しく行うための深度バッファと、特定の計上で描画を切り抜くステンシルバッファの設定
            // Nodeは後から描画したものが手前に上書きされる挙動になる
            depth_stencil: None,

            // ポリゴンの輪郭のギザギザ(ジャギー)を滑らかにするMSAAの設定
            multisample: wgpu::MultisampleState {
                // 1ピクセル当たり何回サンプリングするかを指定する。
                // 1: 無効
                // 4: 4x MSAA
                count: 1,
                // サンプリングする位置をビットマスクで制御する。すべてのサンプルを有効にする場合は`!0`や`1`(countが1の場合)を指定する
                mask: 1,
                // フラグメントシェーダーが出力したアルファ値に応じて、MSAAのサンプル数を動的に変化させる特殊hな透過技術
                // 髪の毛や草木の葉など、半透明テクスチャの輪郭をきれいに描画したい場合にtrueにする
                alpha_to_coverage_enabled: false,
            },

            // VRの左右の目用など、1回の描画コールで複数のレイヤーへ同時にレンダリングを行うマルチビューレンダリングの有効化マスク
            multiview_mask: None,
            // パイプラインのコンパイル結果をキャッシュして次回の起動を高速化するための仕組みを指定する
            cache: None,
        });

        // ─────────────────────────────────────────────────────────────
        // 頂点バッファの作成
        // ─────────────────────────────────────────────────────────────
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vetex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let num_vertices = VERTICES.len() as u32;

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            num_vertices,
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
                
                // 描画先となるカラーテクスチャに関する設定の配列
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // 実際に書き込む対象となるテクスチャのビューを指定する
                    view: &view,
                    // アンチエイリアスを使用している場合に、マルチサンプルされたデータを通常のテクスチャへと短縮・結合する出力先を指定する
                    resolve_target: None,
                    // 3Dテクスチャやテクスチャ配列の特定の層に対して直接レンダリングを行う際、そのインデックスを指定する
                    depth_slice: None,
                    // このレンダーパスの 「開始時（Load）」と「終了時（Store）」に、GPUのメモリに対してどのような操作を行うかを定義する
                    ops: wgpu::Operations {
                        // 描画を始める前に、画面全体を指定した色で塗りつぶしてリセットする
                        // LoadOp::Load: 前のフレーム（または前のパス）で描画された内容をそのままメモリに残した状態で描画を開始する
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        // 
                        store: wgpu::StoreOp::Store,
                    },
                })],

                // 深度テストやステンシルテストを行うための「深度バッファテクスチャ」への接続設定
                depth_stencil_attachment: None,
                // このレンダーパスの「開始時」と「終了時」に、GPU後むスタンプをクエリセットに書きもうための設定
                timestamp_writes: None,
                // 「オクルージョンクエリ(描画した物体が、他の物体に隠されずに実際に何ピクセル画面に描画されたか)」の結果を格納するセットを指定する。
                // 主に画面外や遮蔽物の後ろに隠れて見えないオブジェクトの描画をスキップする最適化技術で使用する。
                occlusion_query_set: None,
                // パイプライン側と同様、VR用のマルチビューレンダリングを行う際のレイヤーマスク。
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            // 頂点バッファをスロット0に割り当てる
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            render_pass.draw(0..self.num_vertices, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
