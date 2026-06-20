// 円周率の定義
const PI: f32 = 3.141592653589793;

// --- ユニフォームバッファの定義 ---
struct Uniforms {
    time: f32,          // アプリケーション起動からの経過時間（秒）
    _pad: f32,          // 16バイトアライメントに合わせるためのパディング
    resolution: vec2<f32>, // 画面の解像度（幅、高さ）
};

// ユニフォームバッファをバインド（Binding 2 に配置）
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

// --- コンピュートシェーダ用バインディング ---
// 1. 前フレームの描画結果（読み取り専用）
@group(0) @binding(0) var last_frame: texture_2d<f32>;

// 2. 今フレームの描画先（書き込み専用ストレージテクスチャ）
@group(0) @binding(1) var current_frame: texture_storage_2d<rgba8unorm, write>;


// コンピュートシェーダのエントリーポイント
// ワークグループのサイズは 16x16 スレッド
@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // 書き込み先テクスチャの解像度を取得
    let texture_size = textureDimensions(current_frame);
    let coords = vec2<i32>(global_id.xy);

    // テクスチャの境界外チェック（領域外への不正書き込みを防止）
    if (coords.x >= i32(texture_size.x) || coords.y >= i32(texture_size.y)) {
        return;
    }

    // 0.0 〜 1.0 のUV座標系に変換
    let uv = vec2<f32>(coords) / vec2<f32>(texture_size);
    
    // 中心が (0,0) で範囲が -1.0 〜 1.0 になるような座標系 (NDC) に変換
    var ndc = uv.xy - vec2f(0.5);
    ndc *= 2.0;

    let aspect = uniforms.resolution.x / uniforms.resolution.y;
    ndc.x *= aspect;

    // --- 1. 前フレームの情報の読み込み ---
    // textureLoad を使うと、サンプラーなしでピクセル座標 (coords) を指定して
    // 前フレームのテクスチャから直接カラー値をロードできます。
    let last_color = textureLoad(last_frame, coords, 0);

    // --- 2. 前フレームの色の減衰（残像効果） ---
    // 毎フレーム、前フレームの色を少しだけ暗く（0.97倍に）します。
    // これにより、過去に描画された円が時間とともに徐々に消えていく残像（トレイル）になります。
    var faded_color = last_color * 0.97;

    // 背景色（初期状態や完全に消えた後の色）は黒（0.0, 0.0, 0.0, 1.0）にします。
    // faded_color のアルファ値は 1.0 を維持するようにします。
    faded_color.a = 1.0;

    // --- 3. 新しい円の位置を計算 ---
    // 時間（uniforms.time）を用いて、円の中心座標を「8の字」軌道で動かします。
    let circle_x = sin(uniforms.time * 2.0) * 0.6 * aspect;
    let circle_y = cos(uniforms.time * 1.2) * 0.4;
    let circle_center = vec2f(circle_x, circle_y);

    // 現在のピクセルから円の中心までの距離を計算
    let dist = length(ndc - circle_center);

    // 円のパラメータ
    let radius = 0.15;      // 半径
    let thickness = 0.005;  // 境界の滑らかさの幅（アンチエイリアス）

    // 円の内部判定（内側が1.0、外側が0.0。アンチエイリアスを考慮して smoothstep で滑らかに繋ぐ）
    let in_circle = 1.0 - smoothstep(radius - thickness, radius + thickness, dist);

    // 円の色（明るい赤色）
    let circle_color = vec4f(1.0, 0.2, 0.2, 1.0);
    
    // --- 4. 前フレームの減衰色と現在の円の色をブレンド ---
    // in_circle が 1.0（円の内側）に近いほど circle_color になり、
    // 0.0（円の外側）に近いほど faded_color（徐々に消えつつある過去の絵）になります。
    let final_color = mix(faded_color, circle_color, in_circle);

    // --- 5. 結果を現在のテクスチャに保存 ---
    // この書き込んだ内容が、次フレームでは「last_frame」としてロードされます。
    textureStore(current_frame, coords, final_color);
}


struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, };
@vertex fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {var out: VertexOutput;let uv = vec2<f32>(f32((in_vertex_index << 1u) & 2u),f32(in_vertex_index & 2u));out.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);out.uv = vec2<f32>(uv.x, 1.0 - uv.y);return out;}
@group(0) @binding(0) var input_texture: texture_2d<f32>; @group(0) @binding(1) var texture_sampler: sampler;
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return textureSample(input_texture, texture_sampler, in.uv); }
