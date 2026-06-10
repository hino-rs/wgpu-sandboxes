const PI: f32 = 3.14159265359;

// const T_MAX: f32 = 512.0;  // クリッピング距離(描画距離)
// const MAX_STEP: u32 = 128; // 最大ステップ(精度)
const MASS: f32 = 1.0;
const HOLE_CENTER: vec3f = vec3f(0.0);
const HOLE_RADIUS: f32 = MASS * 2.0;
const DISK_RADIUS: f32 = HOLE_RADIUS * 4.0;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

struct Uniforms {
    time: f32,
    resolution: vec4f,
    camera_pos: vec4f,
    camera_rot: vec4f,
    params: vec4f, // T_MAX, MAX_STEP
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var sky_texture: texture_2d<f32>;
@group(0) @binding(2) var sky_sampler: sampler;

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
    
    // 閾値設定：上位5%の格子にだけ星を配置する
    if (h > 0.80) {
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
    
    let hole_strength = 0.9 + 0.1 * sin(0.15 * uniforms.time);
    
    // レイマーチングのメインループ
    var t = 0.0;
    var hit = false;
    let dither = hash21(in.uv * uniforms.time);
    var dt = 0.5;
    var ip = ro + rd * dither * dt;
    var glow: f32 = 0.0; // 光をためる
    var id = 0.0;
    
    for (var i = 0u; i < u32(uniforms.params.y); i++) {
        let to_center = HOLE_CENTER - ip;
        let dist_to_center = length(to_center);

        if (dist_to_center < HOLE_RADIUS + 0.01) {
            hit = true;
            break;
        }

        // var max_dt = 0.2;
        // if (dist_to_center > DISK_RADIUS) {
        //     max_dt = dist_to_center * 0.1;
        // }
        // let dt = clamp(dist_to_center * 0.02, 0.005, max_dt);
        


        // let res = map(ip);
        // id = res.y;
        // let d = res.x;

        // if (d < 0.01) {
        //     hit = true;
        //     break;
        // }

        // var to_center = HOLE_CENTER - ip;
        // let dist_to_center = length(to_center);

        
        // let r = length(ip.xz);
        // if (r > HOLE_RADIUS && r < DISK_RADIUS) {
        //     let disk_density = exp(-abs(ip.y) * 4.0) * (1.0 / (r * 0.5));
        //     let gas_vel = normalize(vec3f(-ip.z, 0.0, ip.x));
        //     let doppler = dot(gas_vel, -rd);
            
        //     let d_flop = max(0.0, 1.0 + doppler * 0.7);
        //     let doppler_factor = d_flop * d_flop * d_flop * d_flop;

        //     glow += disk_density * (1.0 + doppler_factor) * dt;
        // }

        // to_center = normalize(to_center);
        
        let bend = normalize(to_center);
        let bend_strength = 3.0 * MASS;
        // rd += bend_strength * (1.0 / (dist_to_center * dist_to_center * dist_to_center)) * bend * dt;
        // rd += hole_strength * (1.0 / (dist_to_center*dist_to_center*dist_to_center)) * bend * dt;
        let dist3 = pow(dist_to_center, 3);
        
        let o_rd = rd;
        rd += bend_strength * (1.0 / dist3) * bend * dt;

        glow += distance(o_rd.z, rd.z) * 0.1;

        let r = length(ip.xz);
        
        // --- 円盤ガス ---
        let theta = atan2(ip.z, ip.x);
        let r_mask = smoothstep(HOLE_RADIUS * 1.2, HOLE_RADIUS * 1.5, r) * smoothstep(DISK_RADIUS, DISK_RADIUS * 0.8, r);
        let y_falloff = exp(-abs(ip.y) * 2.0);
        let disk_mask = r_mask * y_falloff;
        let spiral = theta - (uniforms.time * 2.0 + 10.0) / r;
        let noise_coord = vec2f(r, spiral);
        let n = fbm(noise_coord * 0.5, 4);
        
        let gas_density = disk_mask * n;

        glow += gas_density * dt;

        // --- ジェット ---
        let jet_r = r;
        let jet_y = abs(ip.y);
        let jet_cone = smoothstep(jet_y * 0.2, 0.0, jet_r);
        let jet_falloff = exp(-jet_y * 0.3);
        let jet_mask = jet_cone * jet_falloff;
        let jet_theta = theta;
        let jet_coord = vec2f(jet_theta, ip.y - uniforms.time * 10.0);
        let jet_n = fbm(jet_coord * 0.8, 3);

        glow += jet_mask * n * dt;

        rd = normalize(rd);
        
        ip += rd * dt;
        t += dt;

        if (t > uniforms.params.x) {
            break;
        }
    }

    var color = vec3f(0.0); // 背景色

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
        // color.z += smoothstep(0.0, 1.0, glow);
        // color.z += step(1.0, glow);

        let u = 0.5 + atan2(rd.z, rd.x) / (2.0 * PI);
        let v = 0.5 - asin(rd.y) / PI;
        let sky_color = textureSample(sky_texture, sky_sampler, vec2f(u, v)).rgb;

        // let glow_color = vec3f(0.15 * glow, 0.15 * glow, 0.1 * glow);

        // color = mix(sky_color, glow_color, 0.5);

        

        

        // let noise_mixed_color = mix(mix(vec3(0.2, 0.1, 0.4), vec3(0.8, 0.7, 0.0), n), sky_color, 0.5);
        // let glow_mixed_color = mix(noise_mixed_color, vec3f(glow), 0.5);

        color = mix(sky_color, vec3f(glow), 0.5);
    }

    return vec4f(color, 1.0);
}

fn hash21(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(st: vec2f) -> f32 {
    let i = vec2f(floor(st));
    let f = vec2f(fract(st));
    let a = hash21(i);
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));
    let u = smoothstep(vec2f(0.0), vec2f(1.0), f);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(st: vec2f, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0u; i < octaves; i++) {
        value += amplitude * noise(st * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}