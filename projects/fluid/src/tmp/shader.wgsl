const PI: f32 = 3.14159265358979323846264338327950288;

const DAMPING_AIR: f32 = 0.98;           // 空気抵抗：毎フレーム失われる速度の割合。これがないとエネルギーが溜まり続けて爆発します。
const TIME_STEP: f32 = 0.1;              // タイムステップ：1コマごとの時間の進み幅。

const GRAVITY: vec2f = vec2f(0.0, -0.0012); // 重力：下方向への自然な落下の強さ。
const WALL_BOUNDS: f32 = 0.95;           // 壁の境界：-1.0〜1.0 の正方形の箱の内側を指定。
const WALL_DAMPING: f32 = 0.4;          // 壁の反発係数：壁にぶつかった時にどれだけ勢いが吸収されるか（0.0〜1.0）。

// =========================================================================
// 🌊 Boids / 粒子システム 共通データ構造 (Render Pipeline / Compute Pipeline)
// =========================================================================

// --- 描画パス用のパラメータ群 ---
// CPU(Rust)から Uniform Buffer 経由で毎フレーム送られてくる設定値
struct RenderParams {
    num_boids: u32,      // シミュレーション内の総粒子（ボイド）数
    aspect_ratio: f32,   // 画面のアスペクト比 (幅 / 高さ) - 歪みを防ぐための補正用
    use_trails: u32,     // 残像（トレイル）を描画するかどうかのフラグ (0u: オフ, 1u: オン)
    glow_width: f32,
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
    @location(0) color: vec3f,               // 各頂点に補間されるカラー値 (ピクセルシェーダーへ送る)
    @location(1) local_pos: vec2f,
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

    let max_radius = max(1.0, 0.5 + render_params.glow_width);

    // 1. メッシュの大きさを小さく縮小（0.01倍）し、進行方向へ回転
    // var rotated_pos = rotation * (model.position.xy * 0.02 * size_scale);
    var rotated_pos = rotation * (model.position.xy * 0.02 * size_scale * max_radius);

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
    let color_mid  = vec3f(0.0, 1.0, 0.0);
    // 線形補間（mix）でスピードに合わせた中間色を作る
    // let final_rgb = mix(color_slow, color_fast, t);

    // let final_rgb = select(
    //     mix(color_mid, color_fast, (t - 0.5) * 2.0),
    //     mix(color_slow, color_mid, t * 2.0),
    //     t < 0.5
    // );

    let hue = (240.0 - t * 240.0) / 360.0; // 240度(青)から0度(赤)へマッピング
    let final_rgb = hsv_to_rgb(vec3f(hue, 1.0, 1.0)); 

    // 最終カラー情報をセットしてフラグメントシェーダーへ送る
    out.color = final_rgb;
    // out.local_pos = model.position.xy;

    out.local_pos = model.position.xy * max_radius;

    return out;
}

// HSVからRGBへの変換ヘルパー関数（シェーダーのグローバル領域に定義）
fn hsv_to_rgb(c: vec3f) -> vec3f {
    let K = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3f(0.0), vec3f(1.0)), c.y);
}

// --- フラグメントシェーダー (fs_main) ---
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // --- 円の周りをぼかす ---
    let distance_from_center = length(in.local_pos);
    let circle_radius = 0.5;
    let glow_width = render_params.glow_width;
    let total_radius = circle_radius + glow_width;
    var alpha: f32 = 0.0;

    if (distance_from_center < circle_radius) {
        alpha = 1.0;
    } else if (distance_from_center < total_radius) {
        alpha = 1.0 - smoothstep(circle_radius, total_radius, distance_from_center);
        alpha = alpha * 0.8;
    } else {
        alpha = 0.0;
    }

    return vec4f(in.color, alpha);
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
    let num_particles = arrayLength(&boids_src);

    if (index >= num_particles) { return; }

    let H = params.visual_range;
    let LOOK_AHEAD = params.protected_range;
    let TARGET_DENSITY = params.cohesion_weight * 5.0;
    let PRESSURE_COEF = params.separation_weight * 0.0333;
    let NEAR_PRESSURE_COEF = params.separation_weight * 0.0333;
    let VISCOSITY_COEF = params.alignment_weight * 0.0667;

    var boid = boids_src[index];

    // ---------------------------------------------------------------------
    // 位置予測 (Look-ahead)
    // ---------------------------------------------------------------------
    // 現在の速度から「一コマ未来の位置」を予測し、その場所を基準に物理計算を行います。
    let my_pred_pos = boid.position + boid.velocity * LOOK_AHEAD;

    // ==========================================
    // 予測位置をもとに「密度」と「近接密度」を計算
    // ==========================================
    var density = 0.0;
    var near_density = 0.0;

    for (var i = 0u; i < num_particles; i++) {
        let other = boids_src[i];
        // 相手の未来の予測位置
        let other_pred_pos = other.position + other.velocity * LOOK_AHEAD;
        
        let dst = distance(my_pred_pos, other_pred_pos);

        if (dst < H) {
            // 通常の密度（体積維持用）
            density += smoothing_kernel(dst, H);
            // 近接密度（ダマ防止用・本来はSpikyベースですが現状の関数で代用して蓄積）
            near_density += smoothing_kernel(dst, H);
        }
    }

    // ---------------------------------------------------------------------
    // 状態方程式から「2つの圧力」を計算
    // ---------------------------------------------------------------------
    // 目標密度を超えた分の通常圧力
    let my_pressure = max((density - TARGET_DENSITY) * PRESSURE_COEF, 0.0);
    // 超至近距離用の近接圧力（これは常に反発力として働く）
    let my_near_pressure = near_density * NEAR_PRESSURE_COEF;

    // ==========================================
    // 圧力勾配（反発力）と粘性（摩擦）の計算
    // ==========================================
    var pressure_force = vec2f(0.0, 0.0);
    var viscosity_force = vec2f(0.0, 0.0);

    for (var i = 0u; i < num_particles; i++) {
        if (i == index) { continue; } // 自分自身は除外
        
        let other = boids_src[i];
        let other_pred_pos = other.position + other.velocity * LOOK_AHEAD;
        
        let dir = other_pred_pos - my_pred_pos;
        let dst = length(dir);

        if (dst > 0.0 && dst < H) {
            let dir_normalized = dir / dst;

            // 各種カーネル関数の傾き（導関数）を取得
            let slope = abs(smoothing_kernel_derivative(dst, H));
            let near_slope = abs(spiky_kernel_derivative(dst, H));

            // ---------------------------------------------------------------------
            // 密度による割り算（圧力を加速度へ変換）
            // ---------------------------------------------------------------------
            // 1パス処理のため、自分の圧力情報と傾きを合成し、自分の密度（density）で割ることで
            // 「ギューギューに詰まっている場所ほど、力に対して動きにくくなる」物理特性を再現する。
            let force_magnitude = (my_pressure * slope + my_near_pressure * near_slope) / density;
            
            // 圧力を下げる（相手から離れる）方向に力を加える（-dir_normalized）
            pressure_force += -dir_normalized * force_magnitude;

            // 粘性力（周りの流体と速度を同期させて滑らかにする）
            let weight = smoothing_kernel(dst, H);
            viscosity_force += (other.velocity - boid.velocity) * weight * VISCOSITY_COEF;
        }
    }

    // ==========================================
    // 力の合成と速度の更新
    // ==========================================
    // すべての力（通常圧力 + 近接圧力 + 粘性 + 重力）を現在の速度に加算
    boid.velocity += pressure_force + viscosity_force + GRAVITY;

    // 空気抵抗によるエネルギーの自然減衰
    boid.velocity *= DAMPING_AIR;

    // 速度が無限に加速して破綻するのを防ぐ安全弁
    let max_s = params.max_speed;
    let speed = length(boid.velocity);
    if (speed > max_s) {
        boid.velocity = (boid.velocity / speed) * max_s;
    }

    // ==========================================
    // 位置の更新と壁での跳ね返り判定
    // ==========================================
    boid.position += boid.velocity * TIME_STEP;

    // --- 左右の壁での反発 ---
    if (boid.position.x > WALL_BOUNDS) {
        boid.position.x = WALL_BOUNDS;
        boid.velocity.x *= -WALL_DAMPING;
    } else if (boid.position.x < -WALL_BOUNDS) {
        boid.position.x = -WALL_BOUNDS;
        boid.velocity.x *= -WALL_DAMPING;
    }

    // --- 上下の壁での反発 ---
    if (boid.position.y > WALL_BOUNDS) {
        boid.position.y = WALL_BOUNDS;
        boid.velocity.y *= -WALL_DAMPING;
    } else if (boid.position.y < -WALL_BOUNDS) {
        boid.position.y = -WALL_BOUNDS;
        boid.velocity.y *= -WALL_DAMPING;
    }

    // 計算結果を出力先バッファへ保存
    boids_dst[index] = boid;
}
