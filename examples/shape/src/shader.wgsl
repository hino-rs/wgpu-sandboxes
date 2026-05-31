struct VertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec3f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
    @location(1) uv: vec2f,
    // @location(0) @interpolate(flat) color: vec4f,
}

// Uniformバッファから受け取る構造体
struct TransformUniform {
    position: vec4f,
}

// グループ0のバインディング0番からUniformデータを読み込む
@group(0) @binding(0) var<uniform> u_transform: TransformUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let aspect = u_transform.position.z;
    // X座標だけをアスペクト比で割り算して補正する
    let x = (model.position.x + u_transform.position.x) / aspect;
    let y = model.position.y + u_transform.position.y;
    out.position = vec4f(x, y, 0.0, 1.0);

    out.uv = model.position.xy * 10.0;
    
    out.color = vec4f(model.color, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let dist = length(in.uv);

    if (dist > 1.0) {
        discard;
    }

    return in.color;
}
