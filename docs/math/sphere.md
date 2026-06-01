# 3D球体

## 球体頂点の数式

3次元空間における「球面座標系（極座標系）」から、「直交座標系（x, y, z）」へ位置を変換する公式

- x = r sin Θ cos φ
- y = r cos Θ
- z = r sin Θ sin φ

- r: 半径
- Θ: 軸(この場合y軸)からその点に向かってどれくらい下に傾いているかを表す角度。(緯度に近い)
- φ: 上から見下ろしたときに、基準となる方向から水平にどれくらい回転しているかを表す角度。(経度に近い)

```rust
for lat in 0..=lat_segments {
    for lon in 0..=lon_segments {
        let theta = (lat as f32 / lat_segments as f32) * PI;
        let phi = (lon as f32 / lon_segments as f32) * 2.0 * PI;

        let x = r * theta.sin() * phi.cos();
        let y = r * theta.cos();
        let z = r * theta.sin() * phi.sin();
    }
}
```

> 通常、ポリゴンの法線ベクトル（面の向き）を求めるには複雑な外積計算が必要ですが、原点中心の球体においては「中心から頂点への方向ベクトル」がそのまま法線ベクトルになります。つまり、半径が 1.0（単位球）の場合、計算した位置 [x, y, z] がそのまま法線 [nx, ny, nz] になります。

## テクスチャ座標の計算

- U = 経度ループの進行度 (0.0 ~ 1.0)
- V = 緯度ループの進行度 (0.0 ~ 1.0)

## インデックス (三角形の接続) の構築

頂点グリッドを生成したら、空らを繋いで三角形を作る。

- Latitude(lat): 緯度
- Longitude(lon): 経度

```plaintext
(lat,lon)   (lat,lon+1)
    A --------- B
    |          /|
    |         / |
    |        /  |
    |       /   |
    |      /    |
    |     /     |
    |    /      |
    |   /       |
    |  /        |
    | /         |
    C ----------D
(lat+1,lon)  (lat+1,lon+1)
```

各頂点の一次元インデックスは以下のように求められます：

$$\text{Index} = \text{lat} \times (\text{lon\_segments} + 1) + \text{lon}$$
