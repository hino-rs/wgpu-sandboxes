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
    @location(0) color: vec4f,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // 頂点座標に行列を掛け算して3Dカメラの視点に変換する
    out.position = camera.view_proj * vec4f(model.position, 1.0);
    out.color = vec4f(model.normal * 0.5 + vec3f(0.5), 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}
