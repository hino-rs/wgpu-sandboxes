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
fn map(p: vec3f) -> vec2f {
    var q = p;
    q.x = (fract(q.x / 2.0 + 0.5) - 0.5) * 2.0;
    q.y = (fract(q.y / 2.0 + 0.5) - 0.5) * 2.0;
    // q.z = (fract(q.z / 2.0 + 0.5) - 0.5) * 2.0;

    let sphere_dist = sdf_sphere(q, 0.4);

    return vec2f(sphere_dist, 1.0);
}

// 2つの距離情報をID付きで結合するヘルパー
fn op_union(res1: vec2f, res2: vec2f) -> vec2f {
    if (res1.x < res2.x) {
        return res1; // より距離が近い方を採用
    }
    return res2;
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // UVを[-1.0, 1.0]に変換し、アスペクト比で補正
    let aspect = uniforms.resolution.x / uniforms.resolution.y;
    let p = (in.uv * 2.0 - 1.0) * vec2f(aspect, 1.0);

    // カメラの設定 (レイの開始位置roと方向rd)
    let ro = vec3f(uniforms.camera_pos.xyz);
    var ray_dir = vec3f(p, 1.0);
    ray_dir = rotate_x(ray_dir, uniforms.camera_rot.y); // 上下の回転を適用
    ray_dir = rotate_y(ray_dir, uniforms.camera_rot.x); // 左右の回転を適用
    
    let rd = normalize(ray_dir);

    // レイマーチングのメインループ
    var t = 0.0;
    let t_max = 100.0;
    var hit = false;
    var ip = vec3f(0.0);
    var hit_id = 0.0; // 衝突したオブジェクトのIDを記録する変数

    for (var i = 0u; i < 256u; i++) {
        ip = ro + rd * t;
        let res = map(ip);
        let d = res.x;
        if (d < 0.01) {
            hit = true;
            hit_id = res.y;
            break;
        }
        t += d;
        if (t > t_max) {
            break;
        }
    }

    var color = vec3f(1.0, 1.0, 1.0); // 背景色

    if (hit) {
        let normal = get_normal(ip);
        let sphere_color = normal * 0.5 + 0.5;
        let light_dir = normalize(vec3f(1.0, 1.0, -1.0));
        let diff = max(dot(normal, light_dir), 0.0);

        var base_color = vec3f(0.0);

        color = vec3f(sphere_color) * (diff + 0.1);
    }

    return vec4f(color, 1.0);
}
