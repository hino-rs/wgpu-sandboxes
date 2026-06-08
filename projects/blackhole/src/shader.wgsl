const T_MAX: f32 = 256.0;  // クリッピング距離(描画距離)
const MAX_STEP: u32 = 256; // 最大ステップ(精度)
const EPSILON: f32 = 0.001; // 衝突判定の閾値
const HOLE_CENTER: vec3f = vec3f(0.0);

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

struct Uniforms {
    time: f32,
    resolution: vec4f,
    camera_pos: vec4f,
    camera_rot: vec4f,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // 巨大な三角形を生成
    let x = f32(i32(in_vertex_index == 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index == 2u) * 4 - 1);

    out.clip_position = vec4f(x, y, 0.0, 1.0);
    // 三角形をUV座標に変換
    out.uv = vec2f(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
}

// 球体のSDF
fn sdf_sphere(p: vec3f, s: f32) -> f32 {
    return length(p) - s;
}

// 2つの値を滑らかに補完して最小値を返す
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

// シーン全体のSDF
fn map(p: vec3f) -> f32 {
    let sphere_dist = sdf_sphere(p, 10.0);

    return sphere_dist;
}

// 法線の計算
fn get_normal(p: vec3f) -> vec3f {
    let e = vec2f(0.001, 0.0);
    return normalize(vec3f(
        map(p + e.xyy) - map(p - e.xyy),
        map(p + e.yxy) - map(p - e.yxy),
        map(p + e.yyx) - map(p - e.yyx),
    ));
}

fn rotate_x(p: vec3f, a: f32) -> vec3f {
    let c = cos(a); 
    let s = sin(a);
    return vec3f(p.x, p.y * c - p.z * s, p.y * s + p.z * c);
}

fn rotate_y(p: vec3f, a: f32) -> vec3f {
    let c = cos(a);
    let s = sin(a);
    return vec3f(p.x * c + p.z * s, p.y, -p.x * s + p.z * c);
}

// 3Dベクトルから0〜1の疑似乱数を返す高精度なハッシュ関数
fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// 星空の色を計算する関数
// ray_dir: 正規化されたレイの方向ベクトル
// time: またたき用
fn get_star_field(ray_dir: vec3<f32>, time: f32) -> vec3<f32> {
    // 空間をグリッド（格子）状に分割する（数値が大きいほど星が小さく、高密度になる）
    let scale = 120.0;
    let p = ray_dir * scale;
    let ip = floor(p); // 格子のインデックス（整数部）
    let fp = fract(p); // 格子内の相対座標（0.0〜1.0）

    // 格子ごとに固有のハッシュ値を取得
    let h = hash31(ip);
    
    // 閾値設定：上位5%の格子にだけ星を配置する
    if (h > 0.95) {
        // 星の配置が均一にならないよう、ハッシュ値を使ってセル内で位置をオフセット
        // （星が格子の境界で切れないよう、セルの中心付近に収まる範囲に制限）
        let offset = vec3<f32>(
            hash31(ip + vec3<f32>(1.0, 0.0, 0.0)),
            hash31(ip + vec3<f32>(0.0, 1.0, 0.0)),
            hash31(ip + vec3<f32>(0.0, 0.0, 1.0))
        ) * 0.4 + 0.3; // 0.3 〜 0.7 の範囲に配置

        // 格子内の現在のピクセルから、星の中心までの距離を計算
        // 3次元空間におけるレイと星の距離を計算（垂直距離）
        let star_pos = ip + offset;
        let dist = length(star_pos - ray_dir * dot(star_pos, ray_dir));
        
        // 星のまたたきを計算 (正弦波を組み合わせてランダム感を出す)
        let twinkle = sin(time * (h * 8.0) + h * 20.0) * 0.4 + 0.6;
        
        // smoothstepで星の輪郭をぼかし、ジャギーを防ぐ
        let star_size = 0.25;
        let star_intensity = smoothstep(star_size, 0.0, dist) * twinkle;
        
        // 星に個性を出す
        let star_color = vec3<f32>(h, fract(h * 10.0), fract(h * 100.0)) * 0.3 + vec3<f32>(0.7);
        
        return star_intensity * star_color;
    }
    
    return vec3<f32>(0.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let aspect = uniforms.resolution.x / uniforms.resolution.y;
    let p = (in.uv * 2.0 - 1.0) * vec2f(aspect, 1.0);

    // カメラの設定 (レイの開始位置roと方向rd)
    let ro = vec3f(uniforms.camera_pos.xyz); // レイの原点
    var ray_dir = vec3f(p, 1.0);  // スクリーン座標からレイ方向を作成
    ray_dir = rotate_x(ray_dir, uniforms.camera_rot.y); // 上下の回転を適用
    ray_dir = rotate_y(ray_dir, uniforms.camera_rot.x); // 左右の回転を適用
    
    var rd = normalize(ray_dir);
    let dt = 0.01;

    let hole_strength = 0.9 + 0.1 * sin(0.15 * uniforms.time);

    // レイマーチングのメインループ
    var t = 0.0;
    var hit = false;
    var ip = vec3f(0.0);
    var glow: f32 = 0.0; // 光をためる

    for (var i = 0u; i < MAX_STEP; i++) {
        // 先端 = 始点 + 方向 * 進んだ距離
        ip = ro + rd * t;

        let res = map(ip);

        let d = res;
        glow = glow + exp(-d * 30.0);
        if (d < 0.01) {
            hit = true;
            break;
        }

        var to_center = HOLE_CENTER - ip;
        let dist_to_center = length(to_center);
        to_center = normalize(to_center);
        let bend = to_center;
        rd += hole_strength * (1.0 / dist_to_center) * bend;
        rd = normalize(rd);
        
        t += d;
        if (t > T_MAX) {
            break;
        }
    }

    var color = vec3f(0.0); // 背景色

    if (hit) {
        color = vec3f(0.0);
    } else {
        color = get_star_field(rd, uniforms.time);
        let bg_gradient = mix(vec3<f32>(0.005, 0.005, 0.02), vec3<f32>(0.0), rd.y * 0.5 + 0.5);
        color += bg_gradient;
    }

    return vec4f(color, 1.0);
}
