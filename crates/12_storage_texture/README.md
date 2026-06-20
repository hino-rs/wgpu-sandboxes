# WebGPU / wgpu におけるストレージテクスチャ (Storage Texture) 解説

このサンプルでは、**コンピュートシェーダ（Compute Shader）**からストレージテクスチャに直接描画データを書き込み、それを**レンダーパイプライン（Render Pipeline）**で画面に描画する一連の流れを実装しています。

ここでは、ストレージテクスチャの概念や wgpu での実装手法について詳しく解説します。

---

## 1. ストレージテクスチャとは？

通常のテクスチャ描画とストレージテクスチャには、以下のような違いがあります。

| 機能 | 通常のテクスチャ (Sampled Texture) | ストレージテクスチャ (Storage Texture) |
| :--- | :--- | :--- |
| **主な用途** | キャラクターのテクスチャ、背景画像など | コンピュートシェーダによるシミュレーション、画像処理フィルタ、レイトレーシング出力など |
| **シェーダでのアクセス** | **読み取り専用** (`textureSample` や `textureLoad`) | **任意のピクセル（テクセル）への直接アクセス・書き込み** (`textureStore`) |
| **サンプラーの使用** | 必要 (バイリニアフィルタリングなど) | 不要 (テクセル座標 `vec2<i32>` で直接指定) |
| **wgpu側での用途フラグ** | `wgpu::TextureUsages::TEXTURE_BINDING` | `wgpu::TextureUsages::STORAGE_BINDING` |

### メリット
- シェーダ内の任意のスレッド（ピクセル）から、バッファのようにインデックス（座標）を指定してデータを**直接書き込む**ことができます。
- レンダーターゲット（Color Attachment）を経由せずに画像を出力できるため、Compute Pipeline の実行結果を直接次の処理に受け渡せます。

---

## 2. システム構成

本サンプルでは、同一の物理テクスチャに対して **Compute Shader で書き込み**を行い、**Fragment Shader で読み込み（サンプリング）**を行うという「リソースの共有」を行っています。

```mermaid
graph TD
    subgraph Host (Rust)
        U[Uniform Buffer] -->|Update time/resolution| Q[Queue]
    end
    
    subgraph GPU Pipelines
        CS[Compute Shader <br/>cs_main] -->|1. Write colors via textureStore| ST[(Storage Texture)]
        ST -->|2. Read & Sample| FS[Fragment Shader <br/>fs_main]
        VS[Vertex Shader <br/>vs_main] -->|Generate Fullscreen Tri| FS
        FS -->|3. Present| SC[Swapchain Surface]
    end
```

---

## 3. Rust (ホスト側) の実装ポイント

### ① テクスチャの用途 (Usage) 指定
Compute での「書き込み」と Render での「読み込み」を両立するため、テクスチャ作成時に双方のフラグを立てます。
```rust
let storage_texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("Storage Texture"),
    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    // 書き込みとサンプリングに対応したフォーマット
    format: wgpu::TextureFormat::Rgba8Unorm,
    // 両方の用途を許可する
    usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
    view_formats: &[],
});
```

### ② バインドグループ・レイアウトの定義
バインドグループでの定義方法も、Compute 用（書き込み）と Render 用（サンプリング）で異なります。

#### A. Compute 側レイアウト (Write-Only)
`ty` に `BindingType::StorageTexture` を指定します。
```rust
wgpu::BindGroupLayoutEntry {
    binding: 0,
    visibility: wgpu::ShaderStages::COMPUTE,
    ty: wgpu::BindingType::StorageTexture {
        access: wgpu::StorageTextureAccess::WriteOnly, // 最も互換性が高い書き込み専用
        format: wgpu::TextureFormat::Rgba8Unorm,
        view_dimension: wgpu::TextureViewDimension::D2,
    },
    count: None,
}
```

#### B. Render 側レイアウト (Sampled)
同じテクスチャビューをバインドしますが、こちら側からは通常のテクスチャとして扱います。
```rust
wgpu::BindGroupLayoutEntry {
    binding: 0,
    visibility: wgpu::ShaderStages::FRAGMENT,
    ty: wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    },
    count: None,
}
```

### ③ ウィンドウリサイズ時の挙動
ウィンドウサイズが変わった場合、画面描画用のスワップチェーンだけでなく、**ストレージテクスチャもリサイズ**し、バインドグループを再構成する必要があります。
本サンプルでは `create_storage_resources` 関数を用いて、リサイズイベント（`WindowEvent::Resized`）発生時にテクスチャ、テクスチャビュー、および Compute/Render それぞれの BindGroup を一括で作り直しています。

---

## 4. WGSL (シェーダ側) の実装ポイント

### Compute Shader 側での書き込み
書き込み先のテクスチャを `texture_storage_2d` として宣言し、`textureStore` 関数で書き込みます。

```wgsl
// write アトリビュートを付与して書き込み専用として宣言
@group(0) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let texture_size = textureDimensions(output_texture);
    let coords = vec2<i32>(global_id.xy);

    // テクスチャ境界外への書き込み防止 (重要)
    if (coords.x >= i32(texture_size.x) || coords.y >= i32(texture_size.y)) {
        return;
    }

    // 計算した色をテクスチャの特定座標に直接保存する
    let color = vec4<f32>(1.0, 0.0, 0.0, 1.0); // 例: 赤
    textureStore(output_texture, coords, color);
}
```

### Fragment Shader 側での読み込み (サンプリング)
通常のサンプリング用テクスチャとしてバインドし、UV座標を用いて描画します。

```wgsl
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // コンピュートシェーダで書き込まれた結果をUVサンプリングして描画
    return textureSample(input_texture, texture_sampler, in.uv);
}
```

---

## 5. 重要な Tips / 注意点

### Uniform バッファのアライメントとサイズ一致
Rust の `struct` と WGSL の `struct` のメモリレイアウトは、アライメント規則によりサイズがズレることがあります。
特に `vec3<f32>` などの型を WGSL 内で使用すると、自動的に 16 バイト境界へアライメントされるため、余分なパディングが発生します。

**【失敗例】**
- Rust側: `time (4B) + padding (12B) + resolution (8B) + padding2 (8B) = 32B`
- WGSL側: `time (4B) + _pad (12B: 16B alignment) + resolution (8B: 8B alignment) + _pad2 (8B) = 48B` (アライメントにより強制的に拡張)
- 結果: `wgpu error: Validation Error (buffer bound size 32 where the shader expects 48)`

**【対策】**
Rust側とWGSL側で、アライメントに左右されないシンプルな 16 バイト構成（あるいは 16 バイトの倍数）に合わせることで、サイズ不一致を防ぐことができます。
```rust
// Rust
struct ShaderUniforms {
    time: f32,
    _pad: f32,          // パディング
    resolution: [f32; 2], // 8バイト
} // 合計16バイト
```
```wgsl
// WGSL
struct Uniforms {
    time: f32,
    _pad: f32,
    resolution: vec2<f32>,
}; // 合計16バイト
```
