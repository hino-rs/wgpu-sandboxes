struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    // @location(0) @interpolate(flat) color: vec4<f32>,
}

// Uniformバッファから受け取る構造体
struct TimeUniform {
    time: f32,
}

// グループ0のバインディング0番からUniformデータを読み込む
@group(0) @binding(0) var<uniform> u_time: TimeUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // GPU側でサイン波を使ってX方向の移動量を計算する
    let offset_x = sin(u_time.time) * 0.5;
    let moved_position = vec3<f32>(
        model.position.x + offset_x,
        model.position.y,
        model.position.z
    );
    out.position = vec4<f32>(moved_position, 1.0);
    out.color = vec4<f32>(model.color, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
