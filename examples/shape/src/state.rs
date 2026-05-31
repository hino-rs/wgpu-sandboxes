use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event::{ElementState, WindowEvent};
use winit::dpi::PhysicalSize;

use crate::shape::Ball;

// 頂点データの中身 (4点分 反時計周り)
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.1, -0.1, 0.0],
        color: [0.3, 0.5, 1.0],
    },
    Vertex {
        position: [0.1, -0.1, 0.0],
        color: [0.3, 0.5, 1.0],
    },
    Vertex {
        position: [0.1, 0.1, 0.0],
        color: [0.3, 0.5, 1.0],
    },
    Vertex {
        position: [-0.1, 0.1, 0.0],
        color: [0.3, 0.5, 1.0],
    },
];

// インデックスデータの中身
const INDICES: &[u16] = &[
    0, 1, 2, // 三角形① (V0 -> V1 -> V2)
    2, 3, 0, // 三角形② (V2 -> V3 -> V0)
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

// Uniformバッファ用
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    positions: [[f32; 4]; 30],
    colors: [[f32; 4]; 30],
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
    // インデックスバッファ用
    index_buffer: wgpu::Buffer,
    // インデックスの個数
    num_indices: u32,
    // GPUに作成するUniformバッファを格納する
    uniform_buffer: wgpu::Buffer,
    // このバッファのシェーダーを`@group(0) @binding(0)`に紐付けるためのバインドグループを格納する
    bind_group: wgpu::BindGroup,
    // オブジェクト
    balls: Vec<Ball>,
    // アスペクト比
    aspect: f32,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let aspect = size.width as f32 / size.height as f32;

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

        let uniform_data = Uniform {
            positions: [[0.0; 4]; 30],
            colors: [[1.0; 4]; 30],
        };
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Uniform, 
                        has_dynamic_offset: false, 
                        min_binding_size: None, 
                    },
                    count: None,
                }
            ]
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                // パイプラインにバインドする`BindGroupLayout`の配列を、バインドのグループのインデックス`@group(n)`順に並べたもの
                bind_group_layouts: &[Some(&bind_group_layout)],
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
                // LineStrip (頂点を線でつなぐ)
                // PointList (頂点を点として描画する)
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
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vetex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let num_vertices = VERTICES.len() as u32;

        // ─────────────────────────────────────────────────────────────
        // インデックスバッファの作成
        // ─────────────────────────────────────────────────────────────
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        let mut balls = Vec::new();
        for i in 0..30 {
            let f = i as f32;
            let pos = glam::Vec2::new(
                (f * 0.17).sin() * 0.5,
                (f * 0.23).cos() * 0.5,
            );

            let vel = glam::Vec2::new(
                (f * 0.11).cos() * 0.005 + 0.002,
                (f * 0.13).sin() * 0.005 + 0.002,
            );

            let color = [
                (f * 0.31).sin().abs() * 0.7 + 0.3,
                (f * 0.37).cos().abs() * 0.7 + 0.3,
                (f * 0.43).sin().abs() * 0.7 + 0.3,
            ];
            balls.push(Ball { pos, vel, color });
        }

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            index_buffer,
            num_indices,
            uniform_buffer,
            bind_group,
            balls,
            aspect,
        }
    }

    pub fn update(&mut self) {
        let radius = 0.1;
        // 全てのボールの物理演算をループで更新！
        for ball in &mut self.balls {
            ball.vel.y -= 0.0003;
            ball.vel.x *= 0.999;
            ball.pos += ball.vel;
            // 左右の壁の判定
            if ball.pos.x > (self.aspect - radius) {
                ball.pos.x = self.aspect - radius;
                ball.vel.x = -ball.vel.x * 0.9;
            } else if ball.pos.x < (-self.aspect + radius) {
                ball.pos.x = -self.aspect + radius;
                ball.vel.x = -ball.vel.x * 0.9;
            }
            // 上下の壁の判定
            if ball.pos.y > (1.0 - radius) {
                ball.pos.y = 1.0 - radius;
                ball.vel.y = -ball.vel.y * 0.8;
                ball.vel.x *= 0.95;
            } else if ball.pos.y < (-1.0 + radius) {
                ball.pos.y = -1.0 + radius;
                ball.vel.y = -ball.vel.y * 0.8;
                ball.vel.x *= 0.95;
            }
        }

        let num_balls = self.balls.len();
        for i in 0..num_balls {
            for j in (i + 1)..num_balls {
                let (left, right) = self.balls.split_at_mut(j);
                let ball_i = &mut left[i];
                let ball_j = &mut right[0];

                let delta = ball_j.pos - ball_i.pos;
                let dist = delta.length();
                let min_dist = radius * 2.0;

                if dist < min_dist {
                    let normal = delta / dist;
                    let overlap = min_dist - dist;

                    ball_i.pos -= normal * (overlap * 0.5);
                    ball_j.pos += normal * (overlap * 0.5);

                    let rv = ball_j.vel - ball_i.vel; // 相対速度
                    let vel_along_normal = rv.dot(normal);

                    if vel_along_normal < 0.0 {
                        let restitution = 0.8; // ボールどうしの反発係数
                        let impulse_scalar = -(1.0 + restitution) * vel_along_normal / 2.0;
                        let impulse = normal * impulse_scalar;

                        // 速度を適用
                        ball_i.vel -= impulse;
                        ball_j.vel += impulse;
                    }
                }
            }
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    match key_event.physical_key {
                        PhysicalKey::Code(KeyCode::Space) => {
                            // 全てのボールを一斉にジャンプ！
                            for ball in &mut self.balls {
                                ball.vel.y = 0.012;
                            }
                            return true;
                        }
                        PhysicalKey::Code(KeyCode::ArrowLeft) => {
                            // 全てのボールに左風を送る
                            for ball in &mut self.balls {
                                ball.vel.x -= 0.005;
                            }
                            return true;
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight) => {
                            // 全てのボールに右風を送る
                            for ball in &mut self.balls {
                                ball.vel.x += 0.005;
                            }
                            return true;
                        }
                        _ => {}
                    }
                }
                false
            }
            _ => false,
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
            
            let mut positions = [[0.0f32; 4]; 30];
            let mut colors = [[0.0f32; 4]; 30];
            
            for (i, ball) in self.balls.iter().enumerate() {
                positions[i] = [ball.pos.x, ball.pos.y, self.aspect, 0.0];
                colors[i] = [ball.color[0], ball.color[1], ball.color[2], 1.0];
            }

            let uniform_data = Uniform { positions, colors };
            // GPUへ一括転送 (ループの外で1回だけ呼ぶ)
            self.queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::bytes_of(&uniform_data),
            );
            // バインドグループとバッファをセット
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // 描画 (最後の引数を 0..1 から 0..30 に変更し、30個分一気に描画させる！)
            render_pass.draw_indexed(0..self.num_indices, 0, 0..30);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.aspect = new_size.width as f32 / new_size.height as f32;

            self.surface.configure(&self.device, &self.config);
        }
    }
}
