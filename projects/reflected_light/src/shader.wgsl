const T_MAX: f32 = 256.0;  // クリッピング距離(描画距離)
const MAX_STEP: u32 = 256; // 最大ステップ(精度)
const EPSILON: f32 = 0.001; // 衝突判定の閾値

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
    let x = f32(i32(in_vertex_index == 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index == 2u) * 4 - 1);
    out.clip_position = vec4f(x, y, 0.0, 1.0);
    out.uv = vec2f(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
}

fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453123);
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

fn up_down_sdf_sphere(p: vec3f, s: f32, speed: f32) -> f32 {
    var pp = p - vec3f(0.0, sin(uniforms.time*speed), 0.0);
    return length(pp) - s;
}

// 光源の現在位置を計算する関数 (軌道周回)
fn get_light_pos() -> vec3f {
    let radius = 1.3;
    let speed = 1.5;
    let angle = uniforms.time * speed;
    return vec3f(cos(angle) * radius, 0.3, sin(angle) * radius);
}

// シーン全体のSDF (x: 距離, y: オブジェクトID)
fn map(p: vec3f) -> vec2f {
    let sphere_dist = up_down_sdf_sphere(p, 0.3, 3.0);

    let light_pos = get_light_pos();
    let light_dist = sdf_sphere(p - light_pos, 0.08);

    if (sphere_dist < light_dist) {
        return vec2f(sphere_dist, 1.0); // ID 1.0: メイン球体
    } else {
        return vec2f(light_dist, 2.0);  // ID 2.0: 光源オブジェクト
    }
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
    let ro = vec3f(uniforms.camera_pos.xyz); // レイの原点
    var ray_dir = vec3f(p, 1.0);  // スクリーン座標からレイ方向を作成
    ray_dir = rotate_x(ray_dir, uniforms.camera_rot.y); // 上下の回転を適用
    ray_dir = rotate_y(ray_dir, uniforms.camera_rot.x); // 左右の回転を適用
    
    let rd = normalize(ray_dir);

    // レイマーチングのメインループ
    var t = 0.0;
    var hit = false;
    var ip = vec3f(0.0);
    var hit_id = 0.0; // 衝突したオブジェクトのIDを記録する変数

    for (var i = 0u; i < MAX_STEP; i++) {
        ip = ro + rd * t;
        let res = map(ip);
        let d = res.x;
        hit_id = res.y;
        if (d < EPSILON) {
            hit = true;
            break;
        }
        t += d;
        if (t > T_MAX) {
            break;
        }
    }

    var color = vec3f(0.1, 0.1, 0.1); // 背景色

    if (hit) {
        if (hit_id > 1.5) {
            // 光源自体は自己発光色 (黄色がかった白) にする
            color = vec3f(1.0, 0.95, 0.8);
        } else {
            // メインの球体は、動的な光源位置から計算した方向ベクトルでライティングする
            let N = get_normal(ip);
            let light_pos = get_light_pos();
            let L = normalize(light_pos - ip);
            // 視線方向 = カメラの位置 - 衝突点
            let V = normalize(uniforms.camera_pos.xyz - ip);
            let H = normalize(L + V);
            let ambient = 0.05; // 環境光の強さ
            let diffuse = max(dot(N, L), 0.0); // 拡散反射強度
            let shininess = 64.0; // ハイライトの鋭さ
            let specular = pow(max(dot(N, H), 0.0), shininess); // 鏡面反射強度
            let object_color = vec3f(0.0, 0.0, 0.0);
            let light_color = vec3f(1.0, 1.0, 1.0);
            let final_color = object_color * (ambient + diffuse * light_color) + specular * light_color;

            color = final_color;
        }
    }

    // let mapped_color = reinhard_simple(color);
    // let mapped_color = reinhard_extended(color, 1.0);
    // let mapped_color = reinhard_luminance(color);
    let mapped_color = ACES_fitted(color);

    return vec4f(mapped_color, 1.0);
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
