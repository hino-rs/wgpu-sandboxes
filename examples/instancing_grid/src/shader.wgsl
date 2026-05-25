struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) instance_position: vec3<f32>, // インスタンスごとの位置オフセット
    @location(3) instance_color: vec3<f32>,
    @location(4) instance_scale: f32,          // インスタンスごとのサイズ
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // 動的スケールを適用
    let scaled_position = model.position * model.instance_scale;
    let world_position = scaled_position + model.instance_position;

    out.position = vec4<f32>(world_position, 1.0);
    out.color = vec4<f32>(model.instance_color, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
