struct VertexInput {
    @location(0) position:          vec3f,
    @location(1) color:             vec3f,
    @location(2) instance_position: vec3f,
    @location(3) instance_color:    vec4f,
    @location(4) instance_scale:    f32,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f
}

@vertex fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let scaled_position = vertex.position * vertex.instance_scale;
    let world_position = scaled_position + vertex.instance_position;

    out.position = vec4f(world_position, 1.0);
    // out.color = vec4f(vertex.instance_color, 1.0);
    out.color = vertex.instance_color;

    return out;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}