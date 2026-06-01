const PI: f32 = 3.14159265358979323846264338327950288;

// =========================================================================
// 🌊 Boids / 粒子システム 共通データ構造 (Render Pipeline / Compute Pipeline)
// =========================================================================

// --- 描画パス用のパラメータ群 ---
// CPU(Rust)から Uniform Buffer 経由で毎フレーム送られてくる設定値
struct RenderParams {
    num_boids: u32,      // シミュレーション内の総粒子（ボイド）数
    aspect_ratio: f32,   // 画面のアスペクト比 (幅 / 高さ) - 歪みを防ぐための補正用
    use_trails: u32,     // 残像（トレイル）を描画するかどうかのフラグ (0u: オフ, 1u: オン)
    _p: u32              // メモリアライメント（アライメントサイズを16バイトの倍数にするためのダミー）
}

// グループ0のバインディング0に Uniform バッファとして登録
@group(0) @binding(0) var<uniform> render_params: RenderParams;

// --- 描画パイプラインへの頂点入力 ---
// メッシュ自体の形状データ（今回は三角形を表すローカル位置座標）
struct VertexInput {
    @location(0) position: vec3f, // 頂点の相対位置座標 (x, y, z)
    @location(1) color: vec4f,    // 頂点のベースカラー (r, g, b, a)
}

// --- インスタンス描画による粒子データ入力 ---
// 各粒子（インスタンス）ごとの個別データ。GPUが頂点データの代わりに自動で割り当てる
struct BoidInput {
    @location(2) boid_pos: vec2f, // 3D空間（ここでは2D）上の粒子の中心位置 (x, y)
    @location(3) boid_vel: vec2f, // 粒子の現在の移動速度ベクトル (vx, vy)
}

// --- 頂点シェーダーからピクセルシェーダーへの受け渡しデータ ---
struct VertexOutput {
    @builtin(position) clip_position: vec4f, // クリップ空間における最終的な頂点位置 (GPU用)
    @location(0) color: vec4f,               // 各頂点に補間されるカラー値 (ピクセルシェーダーへ送る)
}

// =========================================================================
// 🎨 描画パイプライン用シェーダー (Render Pipeline)
// =========================================================================

// --- 頂点シェーダー (vs_main) ---
// インスタンス（粒子）ごとに三角形を描画し、速度方向へ回転・配置する
@vertex fn vs_main(
    model: VertexInput,                                   // 三角形メッシュ自体の頂点データ
    instance: BoidInput,                                  // インスタンスごとの粒子データ（位置・速度）
    @builtin(instance_index) instance_idx: u32            // 何番目のインスタンスかを表すID (残像履歴も含む)
) -> VertexOutput {
    var out: VertexOutput;

    let num_boids = render_params.num_boids;
    let size_scale = 1.0;

    // 【進行方向（速度）への回転処理】
    // 速度ベクトルから atan2 を用いてラジアン角度を求める
    let angle = atan2(instance.boid_vel.y, instance.boid_vel.x);
    // 2D回転行列を構築
    let rotation = mat2x2<f32>(
        cos(angle), -sin(angle),
        sin(angle), cos(angle),
    );

    // 1. メッシュの大きさを小さく縮小（0.01倍）し、進行方向へ回転
    var rotated_pos = rotation * (model.position.xy * 0.02 * size_scale);

    // 2. 画面のアスペクト比でX軸を補正（画面が横長でもアスペクト比で割ることで横伸びする歪みを防ぐ）
    rotated_pos.x = rotated_pos.x / render_params.aspect_ratio;

    // 3. 粒子の中心座標（現在地または過去の履歴位置）を足して、最終的なスクリーン上の位置を計算
    let final_pos = rotated_pos + instance.boid_pos;

    // GPUの座標系（Z=0.0, W=1.0）へマッピングして出力
    out.clip_position = vec4f(final_pos, 0.0, 1.0);

    // 【速度に応じたカラーリング】
    // 粒子の移動スピード（ベクトルの長さ）を計算
    let speed = length(instance.boid_vel);
    let min_s = 0.01; // 最低想定速度
    let max_s = 0.03; // 最高想定速度
    // スピードを min_s〜max_s の範囲で 0.0〜1.0 に正規化（クランプ）
    let t = clamp((speed - min_s) / (max_s - min_s), 0.0, 1.0);

    // 位置が安定しているときは青、安定していないほど赤
    let color_slow = vec3f(0.0, 0.0, 1.0);
    let color_fast = vec3f(1.0, 0.0, 0.0);
    // 線形補間（mix）でスピードに合わせた中間色を作る
    let final_rgb = mix(color_slow, color_fast, t);

    // 最終カラー情報をセットしてフラグメントシェーダーへ送る
    out.color = vec4f(final_rgb, 1.0);

    return out;
}

// --- フラグメントシェーダー (fs_main) ---
// ピクセルごとの最終的な色をそのまま出力する
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}

// --- 粒子（ボイド）のデータ構造 ---
struct Boid {
    position: vec2f, // 2D平面上の位置座標 (x, y)
    velocity: vec2f, // 2D平面上の移動速度 (vx, vy)
}

// --- 物理パラメータのデータ構造 ---
// CPU(Rust)側から Uniform バッファで動的に送られてくる、シミュレーション調整値
struct Params {
    visual_range: f32,     // 仲間を認識できる視野の広さ（これより外の仲間は計算しない）
    protected_range: f32,  // ぶつからないように緊急で避ける「パーソナルスペース」の広さ
    separation_weight: f32,// ルール1：分離（衝突回避）の力の重み
    alignment_weight: f32, // ルール2：整列（群れの平均方向に合わせる）の力の重み
    cohesion_weight: f32,  // ルール3：結合（群れの重心に引き寄せられる）の力の重み
    max_speed: f32,        // 粒子の最大速度の制限値
    min_speed: f32,        // 粒子の最低速度の制限値
}

// --- GPUのストレージバッファと Uniform バッファ ---
// read: 今回の計算の入力元データ（前フレームのデータ）
@group(0) @binding(0) var<storage, read> boids_src: array<Boid>;
// read_write: 今回の計算結果を書き込む出力先データ（次フレームで描画・計算に使われる）
@group(0) @binding(1) var<storage, read_write> boids_dst: array<Boid>;
@group(0) @binding(2) var<uniform> params: Params;


// カーネルの重み関数 (2D用)
fn smoothing_kernel(r: f32, h: f32) -> f32 {
    if (r >= h) { return 0.0; }
    let volume = 6.0 / (PI * pow(h, 4.0));
    return (h - r) * (h - r) * volume;
}

// カーネル関数の勾配の定義
fn smoothing_kernel_derivative(r: f32, h: f32) -> f32 {
    if (r >= h) { return 0.0; }
    let scale = -12.0 / (PI * pow(h, 4.0));
    return (h - r) * scale;
}

// 2D Spiky Kernel の勾配（微分）関数
// 粒子が近づくほど、反発力が (h - r) の2乗で爆発的に鋭く立ち上がる
fn spiky_kernel_derivative(r: f32, h: f32) -> f32 {
    if (r >= h || r <= 0.0) { return 0.0; }
    let scale = -30.0 / (PI * pow(h, 5.0)); // 2D Spikyの正規化係数
    return (h - r) * (h - r) * scale;
}

@compute @workgroup_size(64)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let index = global_id.x;
    // let num_particles = arrayLength(&boids_src) / 16u;
    let num_particles = arrayLength(&boids_src);

    if (index >= num_particles) {
        return;
    }

    var boid = boids_src[index];

    // // 1. トレイル用の過去の残像シフト処理（これはそのまま残します）
    // for (var g = 15u; g > 0u; g = g - 1u) {
    //     boids_dst[index + num_particles * g] = boids_src[index + num_particles * (g - 1u)];
    // }

    // パラメータの設定
    let h = params.visual_range; // 検出視野を「カーネル半径 h」として代用します

    // ==========================================
    // STEP 1: 周囲の「密度（Density）」の計算
    // ==========================================
    var density = 0.0;
    for (var i = 0u; i < num_particles; i++) {
        let other = boids_src[i];
        let dst = distance(boid.position, other.position);
        
        // 距離が h 未満の粒子から受ける「重み」を合計する
        density += smoothing_kernel(dst, h);
    }

    // ==========================================
    // STEP 2: 圧力（反発力）と粘性（摩擦）の計算
    // ==========================================
    var pressure_force = vec2f(0.0, 0.0);
    var viscosity_force = vec2f(0.0, 0.0);

    // 調整用の物理パラメータを UI (params) からリアルタイム同期
    let target_density = params.cohesion_weight * 5.0; // 基準密度 (UIで 1.0~8.0 -> 密度 5.0~40.0)
    let pressure_coef = params.separation_weight * 0.005; // 圧力反発の強さ
    let viscosity_coef = params.alignment_weight * 0.01;  // 粘性(ねっとり感)の摩擦強度

    for (var i = 0u; i < num_particles; i++) {
        if (i == index) { continue; } // 自分自身は除外
        
        let other = boids_src[i];
        let dir = other.position - boid.position;
        let dst = length(dir);

        if (dst > 0.0 && dst < h) {
            let dir_normalized = dir / dst;

            // ① 圧力勾配：自分がギュウギュウなほど、相手から強く遠ざかる方向の力を受ける
            // let slope = smoothing_kernel_derivative(dst, h);
            let slope = spiky_kernel_derivative(dst, h);

            // 密度が target_density を超えた分の圧力
            let my_pressure = max((density - target_density) * pressure_coef, 0.0);
            
            // 圧力を下げる（相手から離れる）方向に力を加える
            pressure_force += -dir_normalized * slope * my_pressure;

            // ② 粘性力：周りの水粒子の速度の平均に合わせようとするねっとりした力
            let weight = smoothing_kernel(dst, h);
            viscosity_force += (other.velocity - boid.velocity) * weight * viscosity_coef;
        }
    }

    // ==========================================
    // STEP 3: 力の合成 と 外力（重力）の適用
    // ==========================================
    // 下に向かう重力（適度な強さに調整）
    let gravity = vec2f(0.0, -0.0003);

    // すべての力を速度に加算する
    boid.velocity += pressure_force + viscosity_force + gravity;

    boid.velocity *= 0.98;

    // --- 速度の安全制限（クランプ） ---
    let max_speed = params.max_speed;
    let speed = length(boid.velocity);
    if (speed > max_speed) {
        boid.velocity = (boid.velocity / speed) * max_speed;
    }

    // ==========================================
    // STEP 4: 位置の更新 と 壁での跳ね返り判定
    // ==========================================
    boid.position += boid.velocity * 0.7; // タイムステップを小さめにする

    // 箱の境界を設定 (2Dの -1.0 〜 1.0 の正方形の箱)
    let bounds = 0.95;       // 少し内側を壁にします
    let damping = 0.5;       // 跳ね返り時に吸収されるエネルギー（0.0〜1.0）

    // 左右の壁での反発
    if (boid.position.x > bounds) {
        boid.position.x = bounds;
        boid.velocity.x *= -damping;
    } else if (boid.position.x < -bounds) {
        boid.position.x = -bounds;
        boid.velocity.x *= -damping;
    }

    // 上下の壁での反発（これで下に水が溜まる！）
    if (boid.position.y > bounds) {
        boid.position.y = bounds;
        boid.velocity.y *= -damping;
    } else if (boid.position.y < -bounds) {
        boid.position.y = -bounds;
        boid.velocity.y *= -damping;
    }

    // 最終的な結果を出力先バッファへ保存
    boids_dst[index] = boid;
}
