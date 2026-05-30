// -----------------------------------------
// 共通データ構造
// -----------------------------------------
struct VertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec3f,
}

struct BoidInput {
    @location(2) boid_pos: vec2f,
    @location(3) boid_vel: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec3f,
}

// -----------------------------------------
// Render Pipeline用シェーダー
// -----------------------------------------
@vertex fn vs_main(model: VertexInput, instance: BoidInput) -> VertexOutput {
    var out: VertexOutput;

    // 速度から角度を求めて回転する
    let angle = atan2(instance.boid_vel.y, instance.boid_vel.x);
    let rotation = mat2x2<f32>(
        cos(angle), -sin(angle),
        sin(angle), cos(angle),
    );

    let rotated_pos = rotation * (model.position.xy * 0.01);
    let final_pos = rotated_pos + instance.boid_pos;

    out.clip_position = vec4f(final_pos, 0.0, 1.0);

    let speed = length(instance.boid_vel);
    let min_s = 0.01;
    let max_s = 0.03;
    let t = clamp((speed - min_s) / (max_s - min_s), 0.0, 1.0);

    let color_slow = vec3f(0.0, 0.2, 1.0);
    let color_fast = vec3f(1.0, 0.0, 0.8);

    out.color = mix(color_slow, color_fast, t);

    return out;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4f(in.color, 1.0);
}

// -----------------------------------------
// Compute Pipeline用シェーダー
// -----------------------------------------
struct Boid {
    position: vec2f,
    velocity: vec2f,
}

struct Params {
    visual_range: f32,
    protected_range: f32,
    separation_weight: f32,
    alignment_weight: f32,
    cohesion_weight: f32,
    max_speed: f32,
    min_speed: f32,
}

@group(0) @binding(0) var<storage, read> boids_src: array<Boid>;
@group(0) @binding(1) var<storage, read_write> boids_dst: array<Boid>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&boids_src)) {
        return;
    }
    var boid = boids_src[index];
    // --- パラメータ調整 ---
    let visual_range = params.visual_range;     // 仲間を検知できる視野の広さ
    let protected_range = params.protected_range; // 衝突を避けるための至近距離
    
    // 各ルールの影響度
    let separation_weight = params.separation_weight;
    let alignment_weight = params.alignment_weight;
    let cohesion_weight = params.cohesion_weight;
    // --- 計算用の一時変数 ---
    var close_dx = 0.0;
    var close_dy = 0.0;
    
    var vel_avg = vec2f(0.0, 0.0);
    var pos_avg = vec2f(0.0, 0.0);
    var neighboring_boids = 0.0;
    // 全てのボイドをループでチェック
    for (var i = 0u; i < arrayLength(&boids_src); i++) {
        if (i == index) { 
            continue; // 自分自身は無視する
        }
        let other = boids_src[i];
        let d = distance(boid.position, other.position);
        // 分離 (Separation) - 近すぎる場合は反発する
        if (d < protected_range) {
            close_dx += boid.position.x - other.position.x;
            close_dy += boid.position.y - other.position.y;
        } 
        // 視野内にいる場合は整列と結合の対象にする
        else if (d < visual_range) {
            pos_avg += other.position;
            vel_avg += other.velocity;
            neighboring_boids += 1.0;
        }
    }
    // 各ルールから生まれる力を合成する
    var steering = vec2f(0.0, 0.0);
    // 1. 分離の力を加算
    steering += vec2f(close_dx, close_dy) * separation_weight;
    // 仲間が近くにいた場合、整列と結合の力を計算
    if (neighboring_boids > 0.0) {
        pos_avg = pos_avg / neighboring_boids; // 重心位置
        vel_avg = vel_avg / neighboring_boids; // 平均速度
        // 2. 結合の力 (重心へ引っ張られる)
        let cohesion_force = (pos_avg - boid.position) * cohesion_weight;
        // 3. 整列の力 (平均速度に合わせる)
        let alignment_force = (vel_avg - boid.velocity) * alignment_weight;
        steering += cohesion_force + alignment_force;
    }
    // 現在の速度に操舵力を適用する（0.05は追従の滑らかさ）
    boid.velocity += steering * 0.05;
    // --- 速度のクランプ ---
    let max_speed = params.max_speed;
    let min_speed = params.min_speed;
    let speed = length(boid.velocity);
    
    if (speed > max_speed) {
        boid.velocity = (boid.velocity / speed) * max_speed;
    } else if (speed < min_speed && speed > 0.0) {
        boid.velocity = (boid.velocity / speed) * min_speed;
    }
    // 位置を更新する
    boid.position += boid.velocity;
    // 画面端のループ処理
    if (boid.position.x > 1.0) { boid.position.x = -1.0; }
    if (boid.position.x < -1.0) { boid.position.x = 1.0; }
    if (boid.position.y > 1.0) { boid.position.y = -1.0; }
    if (boid.position.y < -1.0) { boid.position.y = 1.0; }

    

    boids_dst[index] = boid;
}