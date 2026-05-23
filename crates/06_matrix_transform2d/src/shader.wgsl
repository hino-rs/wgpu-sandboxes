struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    // @location(0) color: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
}

// Uniformバッファから受け取る構造体
struct TransformUniform {
    matrix: mat4x4<f32>,
}

// グループ0のバインディング0番からUniformデータを読み込む
@group(0) @binding(0) var<uniform> u_transform: TransformUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = u_transform.matrix * vec4<f32>(model.position, 1.0);
    out.color = vec4<f32>(model.color, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
