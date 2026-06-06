// --------------------------------------------------------------------
// 1. バインドグループの宣言
// --------------------------------------------------------------------
// @group(X) は set_bind_group(X, ...) に対応します。
// @binding(Y) は BindGroupLayout 内の binding: Y に対応します。
@group(0) @binding(0) var t_diffuse: texture_2d<f32>; // 2Dテクスチャリソース
@group(0) @binding(1) var s_diffuse: sampler;          // サンプラールール

// 頂点シェーダーへの入力
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_cords: vec2f, // 頂点バッファ定義の第2属性（UV座標）
}

// 頂点シェーダーからの出力（ラスタライザでピクセルごとに自動線形補間される）
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_cords: vec2f, // フラグメントシェーダーへ送る補間用UV
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(model.position, 1.0);
    out.color = vec4<f32>(model.color, 1.0);
    // 頂点ごとのUV座標をそのままフラグメントシェーダーへ渡す
    out.tex_cords = model.tex_cords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // --------------------------------------------------------------------
    // 2. テクスチャのサンプリング処理
    // --------------------------------------------------------------------
    // textureSample 関数に「テクスチャ」「サンプラー」「補間されたUV座標」を渡すことで、
    // そのピクセル位置に該当するテクスチャの色 (RGBA) を取り出します。
    return textureSample(t_diffuse, s_diffuse, in.tex_cords);
}
