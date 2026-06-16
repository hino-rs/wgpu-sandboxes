const PI: f32 = 3.14159265359;
const ORIGIN: vec3f = vec3f(0.0, 0.0, 0.0);
const GAS_SIZE: f32 = 30.0;
const GAS_COLOR: vec3f = vec3f(0.01, 0.01, 0.02);

// 光源の位置と色
const LIGHT_POS: vec3f = vec3f(0.0, 0.0, 0.0); // 中心星
const LIGHT_COLOR: vec3f = vec3f(3.0, 2.2, 1.5); // やや温かみのある強力な星の光

// ドメインワーピング（座標の歪み）の計算
fn get_warped_coords(ip: vec3f) -> vec3f {
    // 時間経過でうねりをアニメーションさせる
    let time_offset = vec3f(0.0, 0.0, uniforms.time * 0.05);
    let warp_coord = ip * 0.15 + time_offset;
    
    // 3Dノイズで変位ベクトルを作る (平均が0になるように -0.5 する)
    let warp_offset = vec3f(
        noise3d(warp_coord),
        noise3d(warp_coord + vec3f(11.7, 23.4, 35.1)),
        noise3d(warp_coord + vec3f(47.3, 59.2, 71.1))
    ) - vec3f(0.5);

    let dist_to_center = length(ip);
    let base_density = max(0.0, GAS_SIZE - dist_to_center) / GAS_SIZE;
    
    // 星雲の境界部（base_densityが0に近い場所）ではクリッピングを防ぐため歪みを弱くする
    let warp_strength = 10.0 * base_density;

    return ip + warp_offset * warp_strength;
}

// シャドウ計算用の軽量な密度関数
fn calc_density_shadow(ip: vec3f) -> f32 {
    let warped_ip = get_warped_coords(ip);
    let dist_to_center = length(warped_ip);
    let base_density = max(0.0, GAS_SIZE - dist_to_center) / GAS_SIZE;

    var noise_val = fbm3d(warped_ip * 0.25, 1u);
    noise_val = max(0.0, noise_val - 0.25) * 1.5;

    let density = base_density * noise_val;
    return density * uniforms.density_coef;
}


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
    absorption_coef: f32,
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

// レイと球の交差判定。
// x には侵入点(t_entry)、y には脱出点(t_exit)を返す。
// 交差しない場合は x > y を返す。
fn intersect_sphere(ro: vec3f, rd: vec3f, sphere_center: vec3f, sphere_radius: f32) -> vec2f {
    let oc = ro - sphere_center;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - sphere_radius * sphere_radius;
    let h = b * b - c;
    if (h < 0.0) {
        return vec2f(-1.0, -1.0);
    }
    let sqrt_h = sqrt(h);
    return vec2f(-b - sqrt_h, -b + sqrt_h);
}


fn calc_density(ip: vec3f) -> f32 {
    let warped_ip = get_warped_coords(ip);
    let dist_to_center = length(warped_ip);

    // 球の半径の内側ほど密度を高く
    let base_density = max(0.0, GAS_SIZE - dist_to_center) / GAS_SIZE;

    // 3Dノイズをサンプリング
    var noise_val = fbm3d(warped_ip * 0.25, 4u);
    noise_val = max(0.0, noise_val - 0.25) * 1.5;

    // ベースの球体形状とノイズを掛け算する
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
    let bounds = intersect_sphere(ro, rd, ORIGIN, GAS_SIZE);
    let t_entry = bounds.x;
    let t_exit = bounds.y;

    var glow = vec3f(0.0);
    var transmittance = 1.0;

    if (t_exit > 0.0 && t_entry <= t_exit) {
        let dt = 0.1;
        // 開始地点を交差点(t_entry)にする。ただしカメラがすでに球の内部にいる場合は0.0にする。
        // カラーバンディングを防ぐためにジッターを追加。
        let t_start = max(0.0, t_entry) + hash21(in.uv * uniforms.time) * dt;
        var t = t_start;
        let t_end = min(uniforms.t_max, t_exit);

        for (var i = 0u; i < uniforms.max_steps; i++) {
            let ip = ro + rd * t;
            let density = calc_density(ip);
            
            if (density > 0.0) {
                // 1. 光源への方向と距離を計算
                let light_vec = LIGHT_POS - ip;
                let dist_to_light = length(light_vec);
                let light_dir = normalize(light_vec);
                // 2. シャドウレイのマーチング (光源に向かって進む)
                var shadow_t = 0.0;
                let shadow_steps = 4u; // パフォーマンスのため少ないステップ数で走査
                let shadow_dt = dist_to_light / f32(shadow_steps);
                var shadow_density_sum = 0.0;
                for (var j = 0u; j < shadow_steps; j++) {
                    let shadow_ip = ip + light_dir * shadow_t;
                    shadow_density_sum += calc_density_shadow(shadow_ip);
                    shadow_t += shadow_dt;
                }
                // 3. ビールの法則 (Beer's Law) に従う光源からの透過率
                let light_transmittance = exp(-shadow_density_sum * shadow_dt * uniforms.absorption_coef);
                // 4. 位相関数 (Henyey-Greenstein) の適用
                let g = 0.6; // 前方散乱パラメータ
                let phase = henyey_greenstein(rd, -light_dir, g);
                // 5. カメラへの蓄積光の更新
                let absorption = density * dt * uniforms.absorption_coef;
                transmittance *= (1.0 - absorption);
                // ガス自身の固有カラー
                var gas_color = GAS_COLOR;
                gas_color.r += absorption * 0.5;
                gas_color.b += transmittance * 0.2;
                // 「光源からの散乱光」と「ガス自身の発光(emission)」をブレンド
                let scattered_light = LIGHT_COLOR * light_transmittance * phase * density * dt;
                let emission = gas_color * density * dt;
                // 蓄積光に反映
                glow += (scattered_light + emission) * transmittance;
            }
            if (transmittance < 0.001) {
                break;
            }
            t += dt;
            if (t > t_end) {
                break;
            }
        }
    }

    var color = vec3f(0.0);
    let u = 0.5 + atan2(rd.z, rd.x) / (2.0 * PI);
    let v = 0.5 - asin(rd.y) / PI;
    var sky_color = textureSampleLevel(sky_texture, sky_sampler, vec2f(u, v), 0.0);
    sky_color.r += 0.001;
    sky_color.g += 0.003;
    sky_color.b += 0.003;
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
