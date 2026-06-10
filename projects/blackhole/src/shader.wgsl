const T_MAX: f32 = 256.0;  // クリッピング距離(描画距離)
const MAX_STEP: u32 = 256; // 最大ステップ(精度)
const EPSILON: f32 = 0.001; // 衝突判定の閾値
const MASS: f32 = 2.0;
const HOLE_CENTER: vec3f = vec3f(0.0);
const HOLE_RADIUS: f32 = MASS * 2.0;
const DISK_RADIUS: f32 = HOLE_RADIUS * 4.0;
const DT: f32 = 0.1;

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

// 円盤
fn sdf_disk(p: vec3f) -> f32 {
    let radius = DISK_RADIUS;
    let thickness = 0.001;

    let q = vec2f(length(p.xz), p.y);
    let d = abs(q) - vec2f(radius, thickness * 0.5);

    let radius_dist = length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0);
    
    return radius_dist;
}

// 2つの値を滑らかに補完して最小値を返す
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

// シーン全体のSDF
fn map(p: vec3f) -> vec2f {
    let sphere_dist = sdf_sphere(p, HOLE_RADIUS);
    // let disk_dist = sdf_disk(p);

    // if (sphere_dist < disk_dist) {
        return vec2f(sphere_dist, 1.0);
    // }
    // return vec2f(disk_dist, 2.0);
}

// 法線の計算
fn get_normal(p: vec3f) -> vec3f {
    let e = vec2f(0.001, 0.0);
    return normalize(vec3f(
        map(p + e.xyy).x - map(p - e.xyy).x,
        map(p + e.yxy).x - map(p - e.yxy).x,
        map(p + e.yyx).x - map(p - e.yyx).x,
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
    
    // 閾値設定：上位n%の格子にだけ星を配置する
    if (h > 0.9) {
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

// fn snoise(v: vec3f) {

// }

// fn get_space_color(rd: vec3f) -> vec3f {

// }

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
    // let dt = DT;
    
    let hole_strength = 0.9 + 0.1 * sin(0.15 * uniforms.time);
    
    // レイマーチングのメインループ
    var t = 0.0;
    var hit = false;
    var ip = ro;
    var glow: f32 = 0.0; // 光をためる
    var id = 0.0;
    
    for (var i = 0u; i < MAX_STEP; i++) {
        let to_center = HOLE_CENTER - ip;
        let dist_to_center = length(to_center);

        // ブラックホールに吸い込まれた
        if (dist_to_center < HOLE_RADIUS + 0.01) {
            hit = true;
            break;
        }

        // 中心から離れるほどステップサイズを大きくし、遠方なら早期離脱
        var max_dt = 0.2;
        if (dist_to_center > DISK_RADIUS) {
            max_dt = dist_to_center * 0.1; // 遠くでは大股で歩く
        }
        let dt = clamp(dist_to_center * 0.02, 0.005, max_dt);

        // ディスクの描画処理
        let r = length(ip.xz);
        if (r > HOLE_RADIUS && r < DISK_RADIUS) {
            let disk_density = exp(-abs(ip.y) * 4.0) * (1.0 / (r * 0.5));
            let gas_vel = normalize(vec3f(-ip.z, 0.0, ip.x));
            let doppler = dot(gas_vel, -rd);
            
            let d_flop = max(0.0, 1.0 + doppler * 0.7);
            let doppler_factor = d_flop * d_flop * d_flop * d_flop;

            glow += disk_density * (1.0 + doppler_factor) * dt;
        }

        // 重力による光線の曲げ
        let bend = normalize(to_center);
        let bend_strength = 3.0 * MASS;

        let dist3 = dist_to_center * dist_to_center * dist_to_center;
        
        // 物理的
        rd += bend_strength * (1.0 / dist3) * bend * dt;
        
        // rd += hole_strength * (1.0 / dist_to_center) * bend;
        rd = normalize(rd);
        
        ip += rd * dt;
        t += dt;

        if (t > T_MAX) {
            break;
        }
    }

    var color = vec3f(0.0);

    if (hit) {
        // if (id == 1.0) {
            color = vec3f(0.0);
        // } else {
        //     // color = vec3f(glow, glow, 0.0);
        // }
    } else {
        // color = get_star_field(rd, uniforms.time);
        // let bg_gradient = mix(vec3<f32>(0.005, 0.005, 0.02), vec3<f32>(0.0), rd.y * 0.5 + 0.5);
        // color += bg_gradient;
        // color.x += glow;
        // color.y += glow;
        // // color.z += smoothstep(0.0, 1.0, glow);
        // // color.z += step(1.0, glow);
        // if (glow >= 0.2) {
        //     color.z += glow/2.0;
        // }
        color = get_space_color(rd);
    }

    return vec4f(color, 1.0);
}

fn get_space_color(rd: vec3f) -> vec3f {
    var color = vec3f(0.0);

    // 星の描画：高周波のノイズを鋭くクランプし、powで輝点にする
    var n = max(0.0, snoise(rd * 80.0));
    n = pow(n, 80.0) * 50.0;
    color += vec3f(n);

    // 星雲のようなうねり：ノイズ値を [0.0, 1.0] にマッピングして負の色を防ぐ
    n = snoise(rd * 1.0) * 0.5 + 0.5;
    color += 0.15 * vec3f(n, 0.0, n);
    
    n = snoise(rd * 2.0) * 0.5 + 0.5;
    color += 0.1 * vec3f(0.0, n, 0.0);
    
    n = max(0.0, snoise(rd * 4.0));
    n = pow(n, 2.0);
    color += 0.05 * vec3f(0.0, n, n);

    return color;
}

fn mod289_3(x: vec3f) -> vec3f {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn mod289_4(x: vec4f) -> vec4f {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn permute(x: vec4f) -> vec4f {
    return mod289_4(((x * 34.0) + 1.0) * x);
}

fn taylorInvSqrt(r: vec4f) -> vec4f {
    return 1.79284291400159 - 0.85373472095314 * r;
}

fn snoise(v: vec3f) -> f32 {
    const C = vec2f(1.0 / 6.0, 1.0 / 3.0);
    const D = vec4f(0.0, 0.5, 1.0, 2.0);

    // First corner
    let i = floor(v + dot(v, C.yyy));
    let x0 = v - i + dot(i, C.xxx);

    // Other corners
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);

    let x1 = x0 - i1 + C.xxx;
    let x2 = x0 - i2 + C.yyy;
    let x3 = x0 - D.yyy;

    // Permutations
    let i_mod = mod289_3(i);
    let p = permute(permute(permute(
                i_mod.z + vec4f(0.0, i1.z, i2.z, 1.0)
            ) + i_mod.y + vec4f(0.0, i1.y, i2.y, 1.0)
        ) + i_mod.x + vec4f(0.0, i1.x, i2.x, 1.0));

    // Gradients: 7x7 points over a square, mapped onto an octahedron.
    const n_ = 0.142857142857; // 1.0/7.0
    let ns = n_ * D.wyz - D.xzx;

    let j = p - 49.0 * floor(p * ns.z);

    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_);

    let x = x_ * ns.x + ns.yyyy;
    let y = y_ * ns.x + ns.yyyy;
    let h = 1.0 - abs(x) - abs(y);

    let b0 = vec4f(x.xy, y.xy);
    let b1 = vec4f(x.zw, y.zw);

    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4f(0.0));

    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;

    var p0 = vec3f(a0.xy, h.x);
    var p1 = vec3f(a0.zw, h.y);
    var p2 = vec3f(a1.xy, h.z);
    var p3 = vec3f(a1.zw, h.w);

    // Normalise gradients
    let norm = taylorInvSqrt(vec4f(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
    p0 = p0 * norm.x;
    p1 = p1 * norm.y;
    p2 = p2 * norm.z;
    p3 = p3 * norm.w;

    // Mix final noise value
    var m = max(0.6 - vec4f(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), vec4f(0.0));
    m = m * m;
    
    var n = 42.0 * dot(m * m, vec4f(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));

    n = 0.5 * (n + 1.0);

    return n;
}

// 5層のノイズを重ねて密度の高いディテールを作る
fn fbm3D(p_in: vec3f) -> f32 {
    var value: f32 = 0.0;
    var amplitude: f32 = 0.5;
    var frequency: f32 = 1.0;
    var p = p_in;
    
    // WGSLのループでは、インクリメントは `i += 1` と書きます（i++は不可）
    for (var i: i32 = 0; i < 5; i += 1) {
        // ノイズの寄与を加算
        let n = snoise(p * frequency);
        
        // [-1, 1] の結果を [0, 1] にマッピングして重ねる場合
        value += amplitude * (n * 0.5 + 0.5);
        
        // 次のレイヤーのためのセットアップ
        p += vec3f(10.0, 10.0, 10.0); // 周期パターンの重なり（アーティファクト）を防ぐオフセット
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}
