// =====================================================
// Constants
// =====================================================
const PI: f32 = 3.14159265358979323846264338327950288;
const DAMPING_AIR: f32 = 0.98;           // 空気抵抗：毎フレーム失われる速度の割合。これがないとエネルギーが溜まり続けて爆発します。
const TIME_STEP: f32 = 0.1;              // タイムステップ：1コマごとの時間の進み幅。

const GRAVITY: vec2f = vec2f(0.0, -0.0012); // 重力：下方向への自然な落下の強さ。
const WALL_BOUNDS: f32 = 0.95;           // 壁の境界：-1.0〜1.0 の正方形の箱の内側を指定。
const WALL_DAMPING: f32 = 0.4;          // 壁の反発係数：壁にぶつかった時にどれだけ勢いが吸収されるか（0.0〜1.0）。

// =====================================================
// Structures
// =====================================================
struct RenderParams {
    num_particles: u32,
    aspect_ratio: f32,
    glow_width: f32,
}

struct VertexInput {
    @location(0) position: vec3f, // 頂点の相対位置座標
    @location(1) color: vec4f,
}

struct ParticleInput {
    @location(2) position: vec2f,
    @location(3) velocity: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f, // クリップ空間の最終的な頂点位置
    @location(0) color: vec3f,               // 各頂点に保管されるカラー値
    @location(1) local_position: vec2f,      
}

struct Particle {
    position: vec2f,
    velocity: vec2f,
}

struct ParticlesParams {
    visual_range: f32,
    protected_range: f32,
    separation_weight: f32,
    alignment_weight: f32,
    cohesion_weight: f32,
    max_speed: f32,
    min_speed: f32,
}

// =====================================================
// Bindings
// =====================================================
@group(0) @binding(0) var<storage, read> particles_src: array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_dst: array<Particle>;

@group(0) @binding(2) var<uniform> patricles_params: ParticlesParams;
@group(0) @binding(3) var<uniform> render_params: RenderParams;

// =====================================================
// Utilitys
// =====================================================
fn hsv_to_rgb(c: vec3f) -> vec3f {
    let K = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3f(0.0), vec3f(1.0)), c.y);
}

// =====================================================
// Math
// =====================================================

// -----------------------------------------------------
// Equations
// -----------------------------------------------------

// -----------------------------------------------------
// Functions
// -----------------------------------------------------

// 平滑化カーネル
// 粒子間の距離rが影響半径h未満のときに値を持ち、それ以上のときは0になる。
fn smoothing_kernel(r: f32, h: f32) -> f32 {
    if (r >= h) { return 0.0; }
    let volume = 6.0 / (PI * pow(h, 4.0));
    return (h - r) * (h - r) * volume;
}

// 平滑化カーネルの導関数: smoothing_kernelをrで微分
fn smoothing_kernel_derivative(r: f32, h: f32) -> f32 {
    if (r >= h) { return 0.0; }
    let scale = -12.0 / (PI * pow(h, 4.0));
    return (h - r) * scale;
}

// スパイキーカーネルの導関数
// 粒子が極端に近づいたときの反発力を防ぐため。r <= 0 のケースも0にする。
fn spiky_kernel_derivative(r: f32, h: f32) -> f32 {
    if (r >= h || r <= 0.0) { return 0.0; }
    let scale = -30.0 / (PI * pow(h, 5.0));
    return (h - r) * (h - r) * scale;
}

// =====================================================
// Vertex
// =====================================================
@vertex
fn vs_main(
    model: VertexInput,
    instance: ParticleInput,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let num_particles = render_params.num_particles;
    let size_scale = 0.01;

    let max_radius = max(1.0, 0.5 + render_params.glow_width);
    var pos = model.position.xy * size_scale * max_radius;

    pos.x = pos.x / render_params.aspect_ratio;
    let final_pos = pos + instance.position;

    out.clip_position = vec4f(final_pos, 0.0, 1.0);

    let speed = length(instance.velocity);
    let min_s = patricles_params.min_speed;
    let max_s = patricles_params.max_speed;
    let t = clamp((speed - min_s) / (max_s - min_s), 0.0, 1.0);

    let hue = (240.0 - t * 240.0) / 360.0;
    let final_rgb = hsv_to_rgb(vec3f(hue, 1.0, 1.0));

    out.color = final_rgb;
    out.local_position = model.position.xy * max_radius;
    return out;
}

// =====================================================
// Fragment
// =====================================================
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let distance_from_center = length(in.local_position);
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

// =====================================================
// Compute
// =====================================================
@compute @workgroup_size(128)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_particles = arrayLength(&particles_src);

    if (index >= num_particles) { return; }

    let p = patricles_params;
    let H = p.visual_range;
    let LOOK_AHEAD = p.protected_range;
    let TARGET_DENSITY = p.cohesion_weight * 5.0;
    let PRESSURE_COEF = p.separation_weight * 0.0333;
    let NEAR_PRESSURE_COEF = p.separation_weight * 0.333;
    let VISCOSITY_COEF = p.alignment_weight * 0.0667;

    var particle = particles_src[index];

    // 位置予測
    let my_pred_pos = particle.position + particle.velocity * LOOK_AHEAD;

    // ==========================================
    // 予測位置をもとに「密度」と「近接密度」を計算
    // ==========================================
    var density = 0.0;
    var near_density = 0.0;

    for (var i = 0u; i < num_particles; i++) {
        let other = particles_src[i];
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
        
        let other = particles_src[i];
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
            viscosity_force += (other.velocity - particle.velocity) * weight * VISCOSITY_COEF;
        }
    }

    // ==========================================
    // 力の合成と速度の更新
    // ==========================================
    // すべての力（通常圧力 + 近接圧力 + 粘性 + 重力）を現在の速度に加算
    particle.velocity += pressure_force + viscosity_force + GRAVITY;

    // 空気抵抗によるエネルギーの自然減衰
    particle.velocity *= DAMPING_AIR;

    // 速度が無限に加速して破綻するのを防ぐ安全弁
    let max_s = p.max_speed;
    let speed = length(particle.velocity);
    if (speed > max_s) {
        particle.velocity = (particle.velocity / speed) * max_s;
    }

    // ==========================================
    // 位置の更新と壁での跳ね返り判定
    // ==========================================
    particle.position += particle.velocity * TIME_STEP;

    // --- 左右の壁での反発 ---
    if (particle.position.x > WALL_BOUNDS) {
        particle.position.x = WALL_BOUNDS;
        particle.velocity.x *= -WALL_DAMPING;
    } else if (particle.position.x < -WALL_BOUNDS) {
        particle.position.x = -WALL_BOUNDS;
        particle.velocity.x *= -WALL_DAMPING;
    }

    // --- 上下の壁での反発 ---
    if (particle.position.y > WALL_BOUNDS) {
        particle.position.y = WALL_BOUNDS;
        particle.velocity.y *= -WALL_DAMPING;
    } else if (particle.position.y < -WALL_BOUNDS) {
        particle.position.y = -WALL_BOUNDS;
        particle.velocity.y *= -WALL_DAMPING;
    }

    // 計算結果を出力先バッファへ保存
    particles_dst[index] = particle;
}
