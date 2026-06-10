const T_MAX: f32 = 256.0;
const MAX_STEP: u32 = 256;
const EPSILON: f32 = 0.001;
const MAX_TRAIL_COUNT: u32 = 256;
const DT: f32 = 0.05;

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

fn sdf_sphere(p: vec3f, s: f32) -> f32 {
    return length(p) - s;
}

fn map(p: vec3f) -> f32 {
    let time = uniforms.time;
    
    let current_offset = vec3f(time, asin(sin(time)), 0.0);
    var final_dist = sdf_sphere(p - current_offset, 0.05);
    
    let center_t = clamp(p.x, 0.0, time);
     
    let steps = 4;
    
    for (var i = -steps; i <= steps; i++) {
        let t_sample = center_t + f32(i) * DT;
        
        if (t_sample < 0.0 || t_sample > time) {
            continue;
        }
        
        let past_offset = vec3f(t_sample, asin(sin(t_sample)), 0.0);
        let past_sphere_dist = sdf_sphere(p - past_offset, 0.1);
        
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

fn smin(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
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

    let color = select(
        vec3f(1.0),
        vec3f(0.5, 0.8, 1.0),
        hit,
    );

    return vec4f(color, 1.0);
}