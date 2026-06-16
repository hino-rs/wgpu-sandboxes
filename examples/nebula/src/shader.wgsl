const PI: f32 = 3.14159265359;
const ORIGIN: vec3f = vec3f(0.0, 0.0, 0.0);

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

struct Uniforms {
    time: f32,
    max_dt: f32,
    min_dt: f32,
    _p: f32,
    resolution: vec4f,
    camera_pos: vec4f,
    camera_rot: vec4f,
    t_max: f32,
    max_steps: u32,
    density_coef: f32,
    exposure_coef: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var sky_texture: texture_2d<f32>;
@group(0) @binding(2) var sky_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index == 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index == 2u) * 4 - 1);
    out.clip_position = vec4f(x, y, 0.0, 1.0);
    out.uv = vec2f(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
}

// カメラ回転用
fn rotate_x(p: vec3f, a: f32) -> vec3f {
    let c = cos(a); 
    let s = sin(a);
    return vec3f(p.x, p.y * c - p.z * s, p.y * s + p.z * c);
}

// カメラ回転用
fn rotate_y(p: vec3f, a: f32) -> vec3f {
    let c = cos(a);
    let s = sin(a);
    return vec3f(p.x * c + p.z * s, p.y, -p.x * s + p.z * c);
}

// f32用0~1ハッシュ
fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453123);
}

// vec2用0~1ハッシュ
fn hash21(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// vec3用0~1ハッシュ
fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise2d(st: vec2f) -> f32 {
    let i = floor(st);
    let f = fract(st);
    let a = hash21(i);
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));
    
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn noise3d(p: vec3f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    
    let u = f * f * (3.0 - 2.0 * f);
    
    let a000 = hash31(i + vec3f(0.0, 0.0, 0.0));
    let a100 = hash31(i + vec3f(1.0, 0.0, 0.0));
    let a010 = hash31(i + vec3f(0.0, 1.0, 0.0));
    let a110 = hash31(i + vec3f(1.0, 1.0, 0.0));
    let a001 = hash31(i + vec3f(0.0, 0.0, 1.0));
    let a101 = hash31(i + vec3f(1.0, 0.0, 1.0));
    let a011 = hash31(i + vec3f(0.0, 1.0, 1.0));
    let a111 = hash31(i + vec3f(1.0, 1.0, 1.0));
    
    let mix_x00 = mix(a000, a100, u.x);
    let mix_x10 = mix(a010, a110, u.x);
    let mix_x01 = mix(a001, a101, u.x);
    let mix_x11 = mix(a011, a111, u.x);
    
    let mix_y0 = mix(mix_x00, mix_x10, u.y);
    let mix_y1 = mix(mix_x01, mix_x11, u.y);
    
    return mix(mix_y0, mix_y1, u.z);
}

// 2D Fractal Brownian Motion
fn fbm2d(st: vec2f, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0u; i < octaves; i++) {
        value += amplitude * noise2d(st * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// 3D Fractal Brownian Motion
fn fbm3d(p: vec3f, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0u; i < octaves; i++) {
        value += amplitude * noise3d(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

fn sdf_sphere(p: vec3f, s: f32) -> f32 {
    return length(p) - s;
}

fn map(p: vec3f) -> f32 {
    let sphere_dist = sdf_sphere(p, 10.0);
    return sphere_dist;
}

fn calc_density(ip: vec3f) -> f32 {
    let dist_to_center = length(ip);

    // 球の半径の内側ｈど密度を高く
    let base_density = max(0.0, 10.0 - dist_to_center) / 10.0;

    // 3Dノイズをサンプリング
    var noise_val = fbm3d(ip * 0.25, 4u);
    noise_val = max(0.0, noise_val - 0.25) * 1.5;

    // ベースの球体形状とノイズを掛け算する
    // これにより「基本は球形だけど、ノイズによって千切れたり薄くなったりするガス」が作れる
    let density = base_density * noise_val;

    return density * uniforms.density_coef;
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
    var t = 0.0;
    var hit = false;

    var ip = ro + rd * hash21(in.uv * uniforms.time) * 0.5;
    var glow = vec3f(0.0);

    let dt = 0.1;

    var dist_to_center: f32;

    var transmittance = 1.0; // 光の通る割合

    for (var i = 0u; i < uniforms.max_steps; i++) {
        ip = ro + rd * t;

        let density = calc_density(ip);
        
        if (density > 0.0) {
            // 光がどれくらい遮られたか
            let absorption = density * dt * 0.5;

            // 透過率を減衰させる
            transmittance *= (1.0 - absorption);
            
            // この地点のガスが放つ光の強さを計算
            // ガス自身の基本色（例: 青） × 密度 × ステップ幅
            let step_color = vec3f(0.0, 0.8, 0.95) * density * dt;

            // 「手前にあるガス」に遮られた後の光だけがカメラに届くため、
            // 現在の透過率 (transmittance) を掛けてから glow に加算する
            glow += step_color * transmittance;
        }


        // let res = map(ip);

        // if (res < 0.01) {
        //     hit = true;
        //     break;
        // }

        if (transmittance < 0.001) {
            break;
        }

        t += dt;
        if (t > uniforms.t_max) {
            break;
        }
    }

    var color = vec3f(0.0);
    let u = 0.5 + atan2(rd.z, rd.x) / (2.0 * PI);
    let v = 0.5 - asin(rd.y) / PI;
    let sky_color = textureSampleLevel(sky_texture, sky_sampler, vec2f(u, v), 0.0);
    color = sky_color.rgb * transmittance + glow;

        // // 3Dノイズを交差点ip（3次元空間座標）を使ってサンプリング
        // // 時間経過でアニメーションさせる
        // let noise_coord = ip * 0.2 + vec3f(0.0, 0.0, uniforms.time * 0.2);
        // let n = fbm3d(noise_coord, 4u);
        
        // // 3Dノイズの値に基づいて星雲のような色を作成してブレンド
        // let gas_color = mix(vec3f(0.01, 0.0, 0.05), vec3f(0.0, 0.1, 0.2), n);
        // color = mix(color, gas_color, n * 0.9);
        

    let exposed = color * uniforms.exposure_coef;
    
    // let mapped_color = reinhard_simple(exposed);
    // let mapped_color = reinhard_extended(exposed, 1.0);
    // let mapped_color = reinhard_luminance(exposed);
    let mapped_color = ACES_fitted(exposed);

    return vec4f(mapped_color, 1.0);
}

fn henyey_greenstein(view_dir: vec3f, light_dir: vec3f, g: f32) -> f32 {
    let cos_theta = dot(view_dir, light_dir);
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    
    return (1.0 / (4.0 * 3.14159265)) * (1.0 - g2) / pow(denom, 1.5);
}

// ------------------------------------------------------------
// トーンマッピング用関数
// ------------------------------------------------------------
fn ACES_film(x: f32) -> f32 {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return saturate((x * (a * x + b)) / (x * (c * x + d) + e));
}

const ACES_INPUT_MAT: mat3x3f = mat3x3f(
    vec3f(0.59719, 0.07600, 0.02840),
    vec3f(0.35458, 0.90834, 0.13383),
    vec3f(0.04823, 0.01566, 0.83777),
);

const ACES_OUTPUT_MAT: mat3x3f = mat3x3f(
    vec3f(1.60475 , -0.10210, -0.00327),
    vec3f(-0.53108, 1.10813, -0.07276),
    vec3f(-0.07367, -0.06061, 1.07602),
);

fn RRT_and_ODF_titchener(v: vec3f) -> vec3f {
    let a = v * (v + 0.0245786f) - 0.000090537f;
    let b = v * (0.983729f * v + 0.4329510f) + 0.238081f;
    return a / b;
}

fn ACES_fitted(original: vec3f) -> vec3f {
    // 1. 入力行列を掛ける
    let color1 = ACES_INPUT_MAT * original;
    // 2. RRT（参考レンダリングトランスファ）とODFの近似を適用
    let color2 = RRT_and_ODF_titchener(color1);
    // 3. 出力行列を掛けて元の色空間のバランスに戻す
    let final_color = ACES_OUTPUT_MAT * color2;

    // 0〜1にクランプ
    return saturate(final_color);
}

fn reinhard_simple(color: vec3f) -> vec3f {
    return color / (color + vec3f(1.0));
}

fn reinhard_extended(color: vec3f, max_white: f32) -> vec3f {
    let numerator = color * (1.0 + (color / (max_white * max_white)));
    let denominatro = 1.0 + color;
    return numerator / denominatro;
}

fn reinhard_luminance(color: vec3f) -> vec3f {
    let luma = dot(color, vec3f(0.2126, 0.7152, 0.0722));
    let mapped_luma = luma / (1.0 + luma);
    return color * (mapped_luma /luma);
}
