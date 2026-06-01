struct CameraUniform {
    view_proj: mat4x4f,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) tex_coords: vec2f
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    // 影を付けるために法線を送るようにする
    @location(0) normal: vec3f,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // 頂点座標に行列を掛け算して3Dカメラの視点に変換する
    out.position = camera.view_proj * vec4f(model.position, 1.0);

    // 法線をそのまま次のピクセルシェーダーへ渡す
    out.normal = model.normal;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // 球体のベースの色
    let object_color = vec3f(0.2, 0.6, 1.0);

    // 空間全体のうっすらとした環境光
    let ambient_strength = 0.1;
    let ambient = object_color * ambient_strength;

    // 太陽光の方向を定義
    // 右上から徹前に向かって差し込ませる (長さは一旦1)
    let light_dir = normalize(vec3f(1.0, 1.0, 1.0));

    // 法線を正規化する (補完のズレを防ぐため)
    let normal = normalize(in.normal);

    // 光の方向と法線の内積を計算する
    // maxを使い、マイナスを0.0に丸める
    let diffuse_strength = max(dot(normal, light_dir), 0.0);
    let diffuse = object_color * diffuse_strength;

    // 環境項と反射光を足し合わせて、最終的な色を決定
    let final_color = ambient + diffuse;

    return vec4f(final_color, 1.0);
}
