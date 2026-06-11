const PI: f32 = 3.14159265359;

const MASS: f32 = 3.0;
const HOLE_CENTER: vec3f = vec3f(0.0);
const HOLE_RADIUS: f32 = MASS * 2.0;
const DISK_RADIUS: f32 = HOLE_RADIUS * 8.0;
const MIN_DT: f32 = 0.01;
const MAX_DT: f32 = 0.5;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

struct Uniforms {
    time: f32,
    resolution: vec4f,
    camera_pos: vec4f,
    camera_rot: vec4f,
    params: vec4f, // [T_MAX, MAX_STEP, 0.0, 0.0]
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

// 球体のSDF
fn sdf_sphere(p: vec3f, s: f32) -> f32 {
    return length(p) - s;
}

// シーン全体のSDF
fn map(p: vec3f) -> vec2f {
    let sphere_dist = sdf_sphere(p, HOLE_RADIUS);
    return vec2f(sphere_dist, 1.0);
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

// vec3用0~1ハッシュ関数
fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// vec2用0~1ハッシュ関数
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
    var glow: f32 = 0.0;

    let dynamic_jet_x = clamp(abs(sin(uniforms.time)) * 0.12, 0.1, 0.3);
    var dist_to_center: f32;

    let max_steps = u32(uniforms.params.y);

    for (var i = 0u; i < max_steps; i++) {
        let to_center = HOLE_CENTER - ip;
        dist_to_center = length(to_center);

        // レイが十分ブラックホールの遠方ならスキップ
        if (dist_to_center > DISK_RADIUS * 1.5 && dot(rd, to_center) < 0.0) {
            break;
        }

        let dt = clamp(dist_to_center * 0.1, MIN_DT, MAX_DT);

        if (dist_to_center < HOLE_RADIUS + 0.01) {
            hit = true;
            break;
        }

        let bend = to_center / dist_to_center;
        let bend_strength = 10.0 * MASS;
        let dist3 = dist_to_center * dist_to_center * dist_to_center;
        
        let o_rd = rd;
        rd += bend_strength * (1.0 / dist3) * bend * dt;

        glow += distance(o_rd.z, rd.z) * 0.3; // 少し辺りを照らす

        let r = length(ip.xz);
        
        // --- 円盤ガス ---
        let theta = atan2(ip.z, ip.x);
        let r_mask = smoothstep(HOLE_RADIUS * 1.2, HOLE_RADIUS * 1.5, r) * smoothstep(DISK_RADIUS, DISK_RADIUS * 0.8, r);
        let y_falloff = exp(-abs(ip.y) * 2.0);
        let disk_mask = r_mask * y_falloff;
        if (disk_mask > 0.001) {
            let spiral = theta - (uniforms.time * 2.0 + 10.0) / r;
            let noise_coord = vec2f(r, spiral);
            let n = fbm(noise_coord * 0.5, 3);
            let gas_density = disk_mask * n;
            glow += gas_density * dt;
        }

        // --- ジェット ---
        let jet_r = r;
        let jet_y = abs(ip.y);
        let jet_cone = smoothstep(jet_y * 0.2, 0.0, jet_r);
        let jet_falloff = exp(-jet_y * dynamic_jet_x);
        let jet_mask = jet_cone * jet_falloff;
        if (jet_mask > 0.001) {
            let jet_theta = theta;
            let jet_coord = vec2f(jet_theta, ip.y - uniforms.time * 10.0);
            let jet_n = fbm(jet_coord * 0.8, 2);
            glow += jet_mask * jet_n * dt;
        }
        
        rd = normalize(rd);
        ip += rd * dt;
        t += dt;

        if (t > uniforms.params.x) {
            break;
        }
    }

    var color = vec3f(0.0);

    if (hit) {
        color = vec3f(0.0);
    } else {
        let u = 0.5 + atan2(rd.z, rd.x) / (2.0 * PI);
        let v = 0.5 - asin(rd.y) / PI;
        let sky_color = textureSampleLevel(sky_texture, sky_sampler, vec2f(u, v), 0.0);
        let glow = vec3f(0.5 * glow, 0.5 * glow, 0.3 * glow);
        color = mix(sky_color.rgb, glow, 0.5);
    }

    return vec4f(color, 1.0);
}
