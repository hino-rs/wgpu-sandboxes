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

    // 巨大な三角形を生成
    let x = f32(i32(in_vertex_index == 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index == 2u) * 4 - 1);

    out.clip_position = vec4f(x, y, 0.0, 1.0);
    // 三角形をUV座標に変換
    out.uv = vec2f(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
}

fn cosine_palette(t: f32, a: vec3f, b: vec3f, c: vec3f, d: vec3f) -> vec3f {
    return a + b * cos(6.28318 * (c * t + d));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // UVを[-1.0, 1.0]に変換し、アスペクト比で補正
    let aspect = uniforms.resolution.x / uniforms.resolution.y;
    var p = (in.uv * 2.0 - 1.0) * vec2f(aspect, 1.0);

    let angle = uniforms.camera_rot.x;
    let cos_a = cos(angle);
    let sin_a = sin(angle);
    p = vec2f(p.x * cos_a - p.y * sin_a, p.x * sin_a + p.y * cos_a);

    let zoom = exp(-uniforms.camera_pos.z);
    
    let c = p * zoom + uniforms.camera_pos.xy;

    var z = vec2f(0.0);
    var iter = 0.0;

    for (var i = 0u; i < MAX_ITER; i += 1u) {
        let next_x = z.x * z.x - z.y * z.y + c.x;
        z.y = 2.0 * z.x * z.y + c.y;
        z.x = next_x;

        if (dot(z, z) > 4.0) {
            iter = f32(i);
            break;
        }
    }

    var color = vec3f(0.0);

    if (iter < f32(MAX_ITER)) {
        let log_zn = log(dot(z, z)) / 2.0;
        let nu = log(log_zn / log(2.0)) / log(2.0);
        let smooth_iter = iter + 1.0 - nu;

        let t = smooth_iter * 0.02 + uniforms.time * 0.05;
        let a = vec3f(0.5, 0.5, 0.5);
        let b = vec3f(0.5, 0.5, 0.5);
        let c_param = vec3f(1.0, 1.0, 1.0);
        let d = vec3f(0.0, 0.33, 0.67);
        
        color = cosine_palette(t, a, b, c_param, d);
    } else {
        color = vec3f(0.02, 0.02, 0.05);
    }
    return vec4f(color, 1.0);
}
