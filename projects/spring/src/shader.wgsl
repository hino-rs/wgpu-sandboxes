struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct PosUniform {
    x: f32,
    y: f32,
}

@group(0) @binding(0) var<uniform> pos: PosUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = vec4<f32>(model.position, 1.0);
    out.position.x += pos.x;
    out.position.y += pos.y;

    out.color = vec4<f32>(model.color, 1.0);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

