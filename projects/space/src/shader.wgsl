const MAX_ITER: u32 = 1024;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let aspect = uniforms.resolution.x / uniforms.resolution.y;
    let p = (in.uv * 2.0 - 1.0) * vec2f(aspect, 1.0);
    var color = vec3f(0.2, 0.4, 0.9);

    color = vec3(color*abs(sin(p.y)*uniforms.time));

    // if (abs(fract(cos(p.y*uniforms.time))) < 0.01) {
    //     color = vec3f(1.0);
    // }
    // // X軸（中央の横線）からの距離に基づいて急激に減衰する光を作る
    // let dist_to_line = abs(p.y);
    // let glow = exp(-dist_to_line * 15.0); // 15.0を大きくすると細く鋭い光になります

    // let color = vec3f(0.1, 0.8, 1.0) * glow; // 水色のネオン光

    return vec4f(color, 1.0);
}
