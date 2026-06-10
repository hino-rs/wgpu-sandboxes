const T_MAX: f32 = 256.0;
const MAX_STEP: u32 = 256;
const EPSILON: f32 = 0.001;
const MAX_TRAIL_COUNT: u32 = 64;
const DT: f32 = 0.1;
const SPHERE_RADIUS: f32 = 0.1;

struct Uniforms {
    time: f32, 
    resolution: vec4f, 
    camera_pos: vec4f, 
    camera_rot: vec4f, 
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

@vertex // 画面を埋め尽くしてUVを渡すだけ
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(idx == 1u) * 4 - 1);
    let y = f32(i32(idx == 2u) * 4 - 1);
    out.clip_position = vec4f(x, y, 0.0, 1.0);
    out.uv = vec2f(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
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

fn smin(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

fn sdf_sphere(p: vec3f, s: f32) -> f32 {
    return length(p) - s;
}

fn map(p: vec3f) -> f32 {
    let time = uniforms.time;

    let s_max = mix(sin(time), cos(time), tan(time));
    let s_min = -mix(sin(time), cos(time), tan(time));

    let current_offset = vec3f(cos(time), sin(time), smoothstep(s_min, s_max, sin(time)));
    var final_dist = sdf_sphere(p - current_offset, SPHERE_RADIUS);

    for (var i = 1u; i <= min(u32(uniforms.time*10.0), MAX_TRAIL_COUNT); i++) {
        let past_time = time - f32(i) * DT;

        let past_offset = vec3f(cos(past_time), sin(past_time), smoothstep(s_min, s_max, sin(past_time)));

        // let fade = 1.0 - (f32(i) / f32(TRAIL_COUNT + 1u));
        // let past_radius = SPHERE_RADIUS * fade;
        let past_radius = SPHERE_RADIUS;

        let past_sphere_dist = sdf_sphere(p - past_offset, past_radius);

        final_dist = smin(final_dist, past_sphere_dist, 0.15);
    } 

    return final_dist;
}

fn get_normal(p: vec3f) -> vec3f {
    let e = vec2f(0.001, 0.0);
    return normalize(vec3f(
        map(p + e.xyy) - map(p - e.xyy),
        map(p + e.yxy) - map(p - e.yxy),
        map(p + e.yyx) - map(p - e.yyx),
    ));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let aspect = uniforms.resolution.x / uniforms.resolution.y;
    let p = (in.uv * 2.0 - 1.0) * vec2f(aspect, 1.0);
    let ro = vec3f(uniforms.camera_pos.xyz);
    var ray_dir = vec3f(p, 1.0);
    ray_dir = rotate_x(ray_dir, uniforms.camera_rot.y);
    ray_dir = rotate_y(ray_dir, uniforms.camera_rot.x);
    
    let rd = normalize(ray_dir);
    
    var t = 0.0;
    var hit = false;
    var ip = vec3f(0.0);
    
    for (var i = 0u; i < MAX_STEP; i++) {
        ip = ro + rd * t;
        let res = map(ip);
        
        let d = res;
        if (d < EPSILON) {
            hit = true;
            break;
        }

        t += d;
        if (t > T_MAX) {
            break;
        }
    }

    var color = get_sky_color(rd);

    if (hit) {
        let n = get_normal(ip);

        let light_dir = normalize(vec3f(1.0, 2.0, -1.0));
        let view_dir = -rd;
        let metal_base_color = vec3f(0.9);

        // 鏡面ハイライト
        let half_dir = normalize(light_dir + view_dir);
        let spec_power = 64.0;
        let spec = pow(max(dot(n, half_dir), 0.0), spec_power);

        // 環境マッピング
        let reflect_dir = reflect(rd, n);
        let env_color = get_sky_color(reflect_dir);
    
        // フレネル効果
        // 視線と法線が直交する（輪郭）ほど 1.0 に近づく
        let fresnel = pow(1.0 - max(dot(n, view_dir), 0.0), 4.0);

        // 最終的な色の合成
        // 環境の映り込みをベースに、金属の色を乗せる
        var final_metal = env_color * metal_base_color;
        
        // フレネルで輪郭に空の色（環境光）を強く乗せる
        final_metal = mix(final_metal, env_color, fresnel * 0.5);
        
        // 最後に白いハイライトを足す
        color = final_metal + vec3f(spec * 0.8);
    }
    
    return vec4f(color, 1.0);
}

fn get_sky_color(rd: vec3f) -> vec3f {
    let light_dir = normalize(vec3f(1.0, 0.4, -1.0));

    // 空のグラデーション
    let zenith_color = vec3f(0.15, 0.35, 0.75);  // 真上の濃い青
    let horizon_color = vec3f(0.65, 0.78, 0.95); // 地平線付近の薄い青
    let ground_color = vec3f(0.1, 0.12, 0.15);    // 地平線より下（地面）の暗い色

    // rd.yに応じて色を分ける
    var sky = mix(horizon_color, zenith_color, max(rd.y, 0.0));
    sky = select(
        sky,
        mix(horizon_color, ground_color, clamp(-rd.y * 5.0, 0.0, 1.0)),
        rd.y < 0.0,
    );

    // 太陽の追加
    let sun_dot = max(dot(rd, light_dir), 0.0);

    let sun_disk = pow(sun_dot, 400.0) * 20.0;
    let sun_glow = pow(sun_dot, 8.0) * 0.4;

    let sun_color = vec3f(1.0, 0.9, 0.7);

    return sky + (sun_disk + sun_glow) * sun_color;
}
