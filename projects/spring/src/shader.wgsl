struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
    @location(1) uv: vec2f,
}

struct Uniform {
    x: f32,
    y: f32,
    
    spring_color: vec3f,
    weight_color: vec3f,
}

@group(0) @binding(0) var<uniform> data: Uniform;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    var x = f32(i32(vertex_index & 1u) << 2u) - 1.0;
    var y = f32(i32(vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4f(x, y, 0.0, 1.0);
    out.uv = vec2f(x, y);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // 現在処理中のピクセルの座標P (画面上の位置)
    let p = in.uv;
    
    // オブジェクト（おもり）の中心座標B
    let object_center = vec2f(data.x, data.y);
    
    // ピクセルとオブジェクト中心Bとの距離を計算
    let dist_to_object = distance(p, object_center);
    
    // 距離が指定範囲未満ならオブジェクトの範囲内として塗りつぶす
    if (dist_to_object < 0.08) {
        return vec4f(data.weight_color, 1.0);
    }

    // 原点Aからオブジェクトの中心BまでとピクセルPとの最短距離を計算する
    let a = vec2f(0.0, 0.0); // 線の始点A (原点)
    
    let pa = p - a;             // 始点AからピクセルPへのベクトル (AP)
    let ba = object_center - a; // 始点Aから終点Bへのベクトル (AB)
    
    // ベクトルABに対するベクトルAPの投影比率hを計算する
    // h = (AP・AB) / |AB|^2
    // clamp(h, 0.0, 1.0) により、投影点が線分ABの範囲内（始点と終点の間）に収まるように制限する
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    
    // ピクセルPから、線分上で最も近い点（A + h * AB）への距離を計算する
    // pa - ba * h  =>  (P - A) - (B - A) * h  =>  P - (A + h * AB)
    let dist_to_line = length(pa - ba * h);
    
    // 距離が線の太さ未満なら、線の範囲内として塗りつぶす
    if (dist_to_line < 0.003) {
        return vec4f(data.spring_color, 1.0);
    }
    
    // オブジェクトでも線でもない部分は描画を破棄
    discard;
    return vec4f(0.0);
}
