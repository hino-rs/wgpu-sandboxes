# 組み込み関数

## 数学

- `abs`: 絶対値
- `floor`: 
- `fract`
- `sin`
- `cos`
- `sign`
- `ceil`
- `round`
- `trunc`
- `exp2`
- `log`
- `log2`
- `tan`
- `radians`
- `degrees`
- `acos`
- `asin`
- `atan`
- `pow`
- `sqrt`
- `exp`
- `atan2`
- `inverseSqrt`
- `fma`
- `quantizeToF16`

## ベクトル

- `reflect`
- `cross`
- `distance`
- `dot`
- `normalize`
- `length`
- `refract`
- `faceForward`

## 補完

- `min`
- `max`
- `saturate`
- `mix`
- `clamp`
- `step`
- `smoothstep`

## テクスチャ

- `textureSampleBias`
- `textureSampleGrad`
- `textureSampleCompare`
- `textureSamleCompareLevel`
- `textureDimensions`
- `textureStore`
- `textureSample`
- `textureLoad`
- `textureSampleLevel`
- `textureGather`

## アトミック

- `atomicAdd`
- `atomicLoad`
- `atomicStore`
- `atomicMin`
- `atomicMax`
- `atomicCompareExchangeWeak`

## 微分

- `dpdx`
- `dpdy`
- `fwidth`

## パック・アンパック

- `pack2x16float`
- `pack4x8snorm`
- `pack4x8unorm`
- `pack2x16snorm`
- `pack2x16unorm`
- `unpack系`

## 論理

- `select`
- `all`
- `any`

## 行列

- `transpose`
- `determinant`

## その他

- `workgroupBarrier`: ワークグループ同期
- `arrayLengh`: ストレージ配列の長さ
- `bitcast`: ビット列の型再解釈
- `storageBarrier`: ストレージ同期
