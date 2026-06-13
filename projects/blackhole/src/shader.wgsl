const PI: f32 = 3.14159265359;

const MASS: f32 = 3.0;
const HOLE_CENTER: vec3f = vec3f(0.0);
const HOLE_RADIUS: f32 = MASS * 2.0;
const DISK_RADIUS: f32 = HOLE_RADIUS * 8.0;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

struct Uniforms {
    time: f32,
    min_dt: f32,
    max_dt: f32,
    _p: f32,
    resolution: vec4f,
    camera_pos: vec4f,
    camera_rot: vec4f,
    t_max: f32,
    max_steps: u32,
    bend_strength_coef: f32,
    light_up_coef: f32,
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

// ガス用
fn fbm(st: vec2f, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0u; i < octaves; i++) {
        value += amplitude * noise(st * frequency);
        // value += amplitude * abs(noise((st * frequency)*noise(st * frequency)));
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// const color_inner = vec3f(0.5, 0.8, 1.5);
// const color_outer = vec3f(1.0, 0.2, 0.0);

const color_core = vec3f(3.0, 3.5, 4.5);
const color_mid = vec3f(0.1, 0.5, 3.0);
const color_outer = vec3f(0.02, 0.0, 0.2);

// fn get_g_acceleration(pos: vec3f, dir: vec3f) -> vec3f { 
//     let to_center = HOLE_CENTER - pos;
//     let dist = length(to_center);
    
//     let bend = to_center / dist; 
//     let bend_strength = uniforms.bend_strength_coef * MASS; 
//     let dist3 = dist * dist * dist; 
    
//     return dir + (bend_strength * (1.0 / dist3) * bend); 
// }

fn accel(pos: vec3f, vel: vec3f) -> vec3f {
    let r2 = dot(pos, pos);
    let h = cross(pos, vel);
    let h2 = dot(h, h);
    return -1.5 * uniforms.bend_strength_coef * h2 * pos / pow(max(r2, 1e-4), 2.5);
}

// 円盤面でのケプラー軌道速度と視線方向からドップラー係数を計算する
fn doppler_factor(pos: vec3f, rd: vec3f) -> f32 {
    let r = max(length(pos.xz), 1.0);
    let beta = clamp(sqrt(MASS / r), 0.0, 0.85); // 起動速度
    let vel_dir = normalize(vec3f(-pos.z, 0.0, pos.x)); // 半時計回り接戦
    let gamma = 1.0 / sqrt(1.0 - beta * beta);
    let mu = dot(vel_dir, -rd); // 観測者方向の速度成分
    return 1.0 / (gamma * (1.0 - beta * mu));
}

// 重力赤方偏移(内縁ほど0に近づき暗く赤く)
fn grav_shift(pos: vec3f) -> f32 {
    let r = length(pos);
    return sqrt(max(1.0 - HOLE_RADIUS / r, 0.0));
}

// 温度を色に変換する
fn temp_color(t: f32) -> vec3f {
    let red    = vec3f(1.00, 0.22, 0.05);
    let orange = vec3f(1.00, 0.62, 0.20);
    let white  = vec3f(1.00, 0.95, 0.90);
    let blue   = vec3f(0.70, 0.82, 1.20);
    var c = mix(red, orange, smoothstep(0.0, 0.35, t));
    c = mix(c, white, smoothstep(0.35, 0.7, t));
    c = mix(c, blue,  smoothstep(0.7, 1.0, t));
    return c;
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
    var gas_color = vec3f(0.0);
    var min_r = 1e9;

    var dist_to_center: f32;

    var transmittance = 1.0; // 光の通る割合

    for (var i = 0u; i < uniforms.max_steps; i++) {
        let to_center = HOLE_CENTER - ip;
        dist_to_center = length(to_center);
        min_r = min(min_r, dist_to_center);

        // レイが十分ブラックホールの遠方ならマーチングスキップ
        if (dist_to_center > DISK_RADIUS * 1.5 && dot(rd, to_center) < 0.0) {
            break;
        }

        var dt = clamp((dist_to_center * 0.1 + length(uniforms.camera_pos)*0.01), uniforms.min_dt, uniforms.max_dt);
        // 円盤面かつ円盤の半径内に入っている場合ステップサイズを細かくする
        let r_current = length(ip.xz);
        let in_disk_r = step(HOLE_RADIUS * 1.2, r_current) * step(r_current, DISK_RADIUS);
        let near_plane = exp(-abs(ip.y) * 1.5);
        dt *= mix(1.0, 0.25, in_disk_r * near_plane);

        if (dist_to_center < HOLE_RADIUS + 0.01) {
            hit = true;
            break;
        }

        let prev_rd = rd;

        let p1 = ip;
        let v1 = rd;
        let a1 = accel(p1, v1);
        
        let p2 = p1 + v1 * (dt / 2.0);
        let v2 = v1 + a1 * (dt / 2.0);
        let a2 = accel(p2, v2);
        
        let p3 = p1 + v2 * (dt / 2.0);
        let v3 = v1 + a2 * (dt / 2.0);
        let a3 = accel(p3, v3);

        let p4 = p1 + v3 * dt;
        let v4 = v1 + a3 * dt;
        let a4 = accel(p4, v4);

        let v_blend = (v1 + 2.0 * v2 + 2.0 * v3 + v4) / 6.0;
        let a_blend = (a1 + 2.0 * a2 + 2.0 * a3 + a4) / 6.0;

        // rd += k_blend * dt;

        ip += v_blend * dt;
        rd += a_blend * dt;

        // glow += distance(prev_rd.z, rd.z) * uniforms.light_up_coef; // 少し辺りを照らす

        rd = normalize(rd);
        
        let r = length(ip.xz);
        let color_factor = smoothstep(HOLE_RADIUS * 1.5, DISK_RADIUS * 0.8, r); // 内側なら 0.0、外側なら 1.0
        
        // --- 円盤ガス ---
        // let theta = atan2(ip.z, ip.x);
        let r_mask = smoothstep(HOLE_RADIUS * 1.2, HOLE_RADIUS * 1.5, r) * smoothstep(DISK_RADIUS, DISK_RADIUS * 0.8, r);
        let y_falloff = exp(-abs(ip.y) * 2.0);
        let disk_mask = r_mask * y_falloff;
        if (disk_mask > 0.001) {
            // let spiral = theta - (uniforms.time * 25.0 + 10.0) / r;
            // let noise_coord = vec2f(r, spiral);
            // let n = fbm(noise_coord * 0.5, 6);
            let r_in = HOLE_RADIUS * 1.5;
            let omega = 12.0 * pow(r_in / max(r, 0.1), 1.5);
            let a = uniforms.time * omega;
            let ca = cos(a);
            let sa = sin(a);
            let q = vec2f(ip.x * ca - ip.z * sa, ip.x * sa + ip.z * ca);
            var n = fbm(q * 0.18, 6);
            n = pow(n, 1.5);
            let gas_density = disk_mask * n;

            let light_dir = normalize(HOLE_CENTER - ip);
            let g = 0.6;
            let phase = henyey_greenstein(rd, light_dir, g);

            // var base_color = vec3f(0.0);
            // if (color_factor < 0.2) {
            //     base_color = mix(color_core, color_mid, color_factor / 0.2);
            // } else {
            //     base_color = mix(color_mid, color_outer, (color_factor - 0.2) / 0.8);
            // }
            // let energy_boost = exp((1.0 - color_factor) * 2.5);
            // var scattered_light = base_color * gas_density * phase * energy_boost;

            let temp = clamp(pow((HOLE_RADIUS * 1.5) / max(r, HOLE_RADIUS * 1.5), 0.75), 0.0, 1.0);
            let base_color = temp_color(temp);
            let energy_boost = 1.0 + 8.0 * temp; // 内縁ほど強く発光
            var scattered_light = base_color * gas_density * phase * energy_boost;

            // ドップラー効果と重力赤方偏移
            let dpi = doppler_factor(ip, rd);
            let grav = grav_shift(ip);
            let shift = dpi * grav;
            let BEAM = 3.0;
            scattered_light *= pow(max(shift, 0.0), BEAM);
            scattered_light *= mix(vec3f(1.2, 0.7, 0.4), vec3f(0.5, 0.8, 1.4), smoothstep(0.6, 1.5, shift));

            // let base_color = mix(color_inner, color_outer, color_factor);
            // let scattered_light = base_color * gas_density * phase;
            glow += scattered_light * dt * transmittance;
            let absorption_coef = 4.0; // ガスの不透明さを調整する係数
            transmittance *= exp(-gas_density * dt * absorption_coef);
            if (transmittance < 0.001) {
                break;
            }


            // if ((color_factor > 0.999 || color_factor < 0.2)) {
                // let gas_color = mix(color_inner, color_outer, color_factor);
            //     glow += gas_density * dt * gas_color;
            // } else {
            //     glow += gas_density * dt;
            // // }
            // let opacity = gas_density * dt;
            // let absorption = exp(-opacity);

            // // glow += gas_color * opacity * transmittance;
            // glow += opacity * transmittance * henyey_greenstein(rd, );
            // transmittance *= absorption;

            // if (transmittance < 0.01) { break; }
        }

        // --- ジェット ---
        // let jet_r = r;
        // let jet_y = abs(ip.y);
        // let jet_cone = smoothstep(jet_y * 0.2, 0.0, jet_r);
        // let jet_falloff = exp(-jet_y * 0.05);
        // let jet_mask = jet_cone * jet_falloff;
        // if (jet_mask > 0.001) {
        //     let jet_theta = theta;
        //     let jet_coord = vec2f(jet_theta, ip.y - uniforms.time * 10.0);
        //     let jet_n = fbm(jet_coord * 0.8, 2);
        //     glow += jet_mask * jet_n * dt;
        // }
        
        t += dt;

        if (t > uniforms.t_max) {
            break;
        }
    }

    var color = vec3f(0.0);
    // glow *= vec3f(0.7, 0.7, 0.55);

    // アインシュタインリング
    let photon_r = HOLE_RADIUS * 1.5;
    let ring = smoothstep(photon_r * 1.05, photon_r, min_r) * smoothstep(photon_r * 0.95, photon_r, min_r);
    glow += vec3f(1.0, 0.97, 0.90) * ring * 3.0 * transmittance;
    glow *= vec3f(0.7, 0.7, 0.55);

    if (hit) {
        // color = vec3f(0.0 + glow*vec3f(0.5));
        color = glow;
    } else {
        let u = 0.5 + atan2(rd.z, rd.x) / (2.0 * PI);
        let v = 0.5 - asin(rd.y) / PI;
        let sky_color = textureSampleLevel(sky_texture, sky_sampler, vec2f(u, v), 0.0);
        
        // color = mix(sky_color.rgb, glow, 0.5);
        color = sky_color.rgb * transmittance + glow;
    }

    // let mapped_color = reinhard_simple(color);
    // let mapped_color = reinhard_extended(color, 1.0);
    // let mapped_color = reinhard_luminance(color);
    // let mapped_color = ACES_fitted(color);
    
    let exposed = color * 1.5;
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
