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

struct Square {
    puddle: f32,
    depth: f32,
}

struct InstanceRaw {
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
    color_a: f32,
    scale: f32,
}

@group(0) @binding(0) var<storage, read> current_squares: array<Square>;
@group(0) @binding(1) var<storage, read_write> next_squares: array<Square>;
@group(0) @binding(2) var<storage, read_write> instances: array<InstanceRaw>;

struct BrushUniform {
    pos_x: f32,
    pos_y: f32,
    radius: f32,
    strength: f32,
    is_active: u32,
}
@group(0) @binding(3) var<uniform> brush: BrushUniform;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    let index = id.x;
    if (index >= arrayLength(&current_squares)) {
        return;
    }
    let num_grid = 256u;
    let x = index % num_grid;
    let y = index / num_grid;
    // パラメータ
    let flow_rate = 0.5;
    let absorption = 0.01;
    let current_cell = current_squares[index];
    
    // 自然蒸発を適用したベースの水量
    var next_puddle = current_cell.puddle * 0.999; 
    
    // ブラシ（マウスクリック）による水の注入をGPU側で直接処理
    if (brush.is_active == 1u) {
        let dx_b = f32(x) - brush.pos_x;
        let dy_b = f32(y) - brush.pos_y;
        let dist_sq = dx_b * dx_b + dy_b * dy_b;
        if (dist_sq <= brush.radius * brush.radius) {
            next_puddle = min(1.0, next_puddle + brush.strength);
        }
    }
    
    // 流出
    let lowest_idx_for_me = get_lowest_neighbor_index(x, y, num_grid);
    
    if (lowest_idx_for_me == index) {
        // 自分が一番低い場合蒸発させる
        if (next_puddle > 0.0) {
            next_puddle = max(0.0, next_puddle - (current_cell.puddle * absorption));
        }
    } else {
        // 自分より低い場所がある場合流出させる
        if (next_puddle > 0.0) {
            let my_height = current_cell.depth + current_cell.puddle;
            let target_cell = current_squares[lowest_idx_for_me];
            let target_height = target_cell.depth + target_cell.puddle;
            let height_diff = my_height - target_height;
            let target_flow = (height_diff * 0.5) * flow_rate;
            let flow = min(current_cell.puddle, target_flow);
            if (flow > 0.001) {
                next_puddle = next_puddle - flow;
            }
        }
    }
    
    // 流入
    let dx = array<i32, 4>(0, -1, 1, 0);
    let dy = array<i32, 4>(-1, 0, 0, 1);
    for (var d: u32 = 0u; d < 4u; d = d + 1u) {
        let nx = i32(x) + dx[d];
        let ny = i32(y) + dy[d];
        // 隣人がグリッド内かチェック
        if (nx >= 0 && nx < i32(num_grid) && ny >= 0 && ny < i32(num_grid) && ny >= 0) {
            let neighbor_idx = u32(ny) * num_grid + u32(nx);
            
            // 隣人にとって「最も低いマス」が「自分(index)」であるか判定する
            let lowest_for_neighbor = get_lowest_neighbor_index(u32(nx), u32(ny), num_grid);
            
            if (lowest_for_neighbor == index) {
                // 隣人から自分に水が流れてくる
                let neighbor = current_squares[neighbor_idx];
                
                if (neighbor.puddle > 0.0) {
                    let n_height = neighbor.depth + neighbor.puddle;
                    let my_height = current_cell.depth + current_cell.puddle;
                    let height_diff = n_height - my_height;
                    let target_flow = (height_diff * 0.5) * flow_rate;
                    let flow = min(neighbor.puddle, target_flow);
                    if (flow > 0.001) {
                        next_puddle = next_puddle + flow;
                    }
                }
            }
        }
    }

    // 保存
    next_squares[index].puddle = next_puddle;
    next_squares[index].depth = current_squares[index].depth; // 地形はそのままコピー

    // 描画用のインスタンスデータの書き出し
    let cell_pitch = 2.0 / (f32(num_grid) - 1.0);
    let x_pos = f32(x) * cell_pitch - 1.0;
    let y_pos = f32(y) * cell_pitch - 1.0;

    // 色ブレンド計算
    let terrain_color = vec4f(0.4, 0.3, 0.2, 1.0);
    let water_shallow  = vec4f(0.0, 0.7, 0.8, 0.3);
    let water_deep     = vec4f(0.0, 0.1, 0.5, 0.9);
    var final_color = terrain_color;
    if (next_puddle > 0.005) {
        let t = clamp(next_puddle, 0.0, 1.0);
        let water_color = mix(water_shallow, water_deep, t);
        final_color = mix(terrain_color, water_color, water_color.a);
        final_color.w = 1.0; // 不透明にする
    }
    // instances バッファに直接出力
    instances[index].pos_x = x_pos;
    instances[index].pos_y = y_pos;
    instances[index].pos_z = 0.0;
    instances[index].color_r = final_color.x;
    instances[index].color_g = final_color.y;
    instances[index].color_b = final_color.z;
    instances[index].color_a = final_color.w;
    instances[index].scale = cell_pitch;
}

// 指定した座標の周囲で一番高さが低いマスのインデックスを返す
fn get_lowest_neighbor_index(cx: u32, cy: u32, num_grid: u32) -> u32 {
    var lowest_x = cx;
    var lowest_y = cy;
    let current_idx = cy * num_grid + cx;
    let current_cell = current_squares[current_idx];
    var lowest_height = current_cell.depth + current_cell.puddle;

    let dx = array<i32, 4>(0, -1, 1, 0);
    let dy = array<i32, 4>(-1, 0, 0, 1);

    for (var d: u32 = 0u; d < 4u; d = d + 1u) {
        let nx = i32(cx) + dx[d];
        let ny = i32(cy) + dy[d];

        // 境界チェック
        if (nx >= 0 && nx < i32(num_grid)
            && ny < i32(num_grid) && ny >= 0)
        {
            let n_idx = u32(ny) * num_grid + u32(nx);
            let neighbor = current_squares[n_idx];
            let neighbor_height = neighbor.depth + neighbor.puddle;

            if (neighbor_height < lowest_height) {
                lowest_height = neighbor_height;
                lowest_x = u32(nx);
                lowest_y = u32(ny);
            }
        }
    }

    return lowest_y * num_grid + lowest_x;
}