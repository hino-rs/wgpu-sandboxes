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

// シーン全体のSDF
fn map(p: vec3f) -> f32 {
    // 時間経過で球体を上下に動かす
    let offset = vec3f(0.0, sin(uniforms.time * 2.0) * 0.5, 0.0);
    return sdf_sphere(p - offset, 1.0);
}

// 法線の計算
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
    // UVを[-1.0, 1.0]に変換し、アスペクト比で補正
    let aspect = uniforms.resolutin.x / uniforms.resolutin.y;
    let p = (in.uv * 2.0 - 1.0) * vec2f(aspect, -1.0);

    // カメラの設定 (レイの開始位置roと方向rd)
    let ro = vec3f(0.0, 0.0, -3.0);
    let rd = normalize(vec3f(p, 1.0));

    // レイマーチングのメインループ
    var t = 0.0;
    let t_max = 20.0;
    var hit = false;
    var ip = vec3f(0.0); // 衝突点

    for (var i = 0u; i < 80u; i++) {
        ip = ro + rd * t;
        let d = map(ip);
        if (d < 0.001) {
            hit = true;
            break;
        }
        t += d;
        if (t > t_max) {
            break;
        }
    }

    // 色の計算
    var color = vec3f(0.1, 0.1, 0.15);

    if (hit) {
        let normal = get_normal(ip);
        let light_dir = normalize(vec3f(1.0, 1.0, -1.0));

        // ディフューズ(拡散光)ライティング
        let diff = max(dot(normal, light_dir), 0.0);

        // 球体のベースカラー
        let base_color = vec3f(0.9, 0.3, 0.2);
        color = base_color * (diff + 1); // 環境光成分0.1を加算
    }

    return vec4f(color, 1.0);
}