struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

struct Uniforms {
    time: f32,
    resolutin: vec3f,
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

// 床のSDF
fn sdf_plane(p: vec3f, h: f32) -> f32 {
    return p.y - h; // Y座標がhの無限平面
}

// シーン全体のSDF (距離とマテリアルIDを返す)
fn map(p: vec3f) -> vec2f {
    let plane_dist = sdf_plane(p, -1.0);
    let plane_res = vec2f(plane_dist, 1.0); // ID=1.0

    let sphere_dist = sdf_sphere(p - vec3f(0.0, sin(uniforms.time * 2.0) * 0.5, 0.0), 1.0);
    let sphere_res = vec2f(sphere_dist, 2.0); // ID=2.0

    return op_union(plane_res, sphere_res);
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // UVを[-1.0, 1.0]に変換し、アスペクト比で補正
    let aspect = uniforms.resolutin.x / uniforms.resolutin.y;
    let p = (in.uv * 2.0 - 1.0) * vec2f(aspect, 1.0);

    // カメラの設定 (レイの開始位置roと方向rd)
    let ro = vec3f(0.0, 0.0, -3.0);
    let rd = normalize(vec3f(p, 1.0));

    // レイマーチングのメインループ
    var t = 0.0;
    let t_max = 20.0;
    var hit = false;
    var ip = vec3f(0.0);
    var hit_id = 0.0; // 衝突したオブジェクトのIDを記録する変数

    for (var i = 0u; i < 80u; i++) {
        ip = ro + rd * t;
        let res = map(ip);
        let d = res.x;
        if (d < 0.001) {
            hit = true;
            hit_id = res.y;
            break;
        }
        t += d;
        if (t > t_max) {
            break;
        }
    }

    var color = vec3f(0.1, 0.1, 0.15); // 背景色

    if (hit) {
        let normal = get_normal(ip);
        let light_dir = normalize(vec3f(1.0, 1.0, -1.0));
        let diff = max(dot(normal, light_dir), 0.0);

        var base_color = vec3f(0.0);

        if (hit_id == 1.0) {
            // 床のマテリアル (チェッカーボード模様)
            // 衝突位置のXとZ座標からグリッドを計算
            let check = (f32(i32(floor(ip.x * 2.0)) + i32(floor(ip.z * 2.0))) % 2.0);
            if (check == 0.0) {
                base_color = vec3f(0.3, 0.3, 0.35); // 濃いグレー
            } else {
                base_color = vec3f(0.5, 0.5, 0.55); // 薄いグレー
            }
        } else if (hit_id == 2.0) {
            base_color = vec3f(0.9, 0.3, 0.2); // オレンジ
        }

        color = base_color * (diff + 0.1);
    }

    return vec4f(color, 1.0);
}
