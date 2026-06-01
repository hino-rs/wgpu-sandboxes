// =========================================================================
// 🌊 Boids / 粒子システム 共通データ構造 (Render Pipeline / Compute Pipeline)
// =========================================================================

// --- 描画パス用のパラメータ群 ---
// CPU(Rust)から Uniform Buffer 経由で毎フレーム送られてくる設定値
struct RenderParams {
    num_boids: u32,      // シミュレーション内の総粒子（ボイド）数
    aspect_ratio: f32,   // 画面のアスペクト比 (幅 / 高さ) - 歪みを防ぐための補正用
    use_trails: u32,     // 残像（トレイル）を描画するかどうかのフラグ (0u: オフ, 1u: オン)
    _p: u32              // メモリアライメント（アライメントサイズを16バイトの倍数にするためのダミー）
}

// グループ0のバインディング0に Uniform バッファとして登録
@group(0) @binding(0) var<uniform> render_params: RenderParams;

// --- 描画パイプラインへの頂点入力 ---
// メッシュ自体の形状データ（今回は三角形を表すローカル位置座標）
struct VertexInput {
    @location(0) position: vec3f, // 頂点の相対位置座標 (x, y, z)
    @location(1) color: vec4f,    // 頂点のベースカラー (r, g, b, a)
}

// --- インスタンス描画による粒子データ入力 ---
// 各粒子（インスタンス）ごとの個別データ。GPUが頂点データの代わりに自動で割り当てる
struct BoidInput {
    @location(2) boid_pos: vec2f, // 3D空間（ここでは2D）上の粒子の中心位置 (x, y)
    @location(3) boid_vel: vec2f, // 粒子の現在の移動速度ベクトル (vx, vy)
}

// --- 頂点シェーダーからピクセルシェーダーへの受け渡しデータ ---
struct VertexOutput {
    @builtin(position) clip_position: vec4f, // クリップ空間における最終的な頂点位置 (GPU用)
    @location(0) color: vec4f,               // 各頂点に補間されるカラー値 (ピクセルシェーダーへ送る)
}

// =========================================================================
// 🎨 描画パイプライン用シェーダー (Render Pipeline)
// =========================================================================

// --- 頂点シェーダー (vs_main) ---
// インスタンス（粒子）ごとに三角形を描画し、速度方向へ回転・配置する
@vertex fn vs_main(
    model: VertexInput,                                   // 三角形メッシュ自体の頂点データ
    instance: BoidInput,                                  // インスタンスごとの粒子データ（位置・速度）
    @builtin(instance_index) instance_idx: u32            // 何番目のインスタンスかを表すID (残像履歴も含む)
) -> VertexOutput {
    var out: VertexOutput;

    let num_boids = render_params.num_boids;
    
    // 【残像（トレイル）の世代計算】
    // インスタンスインデックスを総数で割ることで、この頂点が「何フレーム過去のものか」を割り出す
    // 0: 現在, 1: 1フレーム過去, 2: 2フレーム過去 ... 15: 15フレーム過去
    let generation = f32(instance_idx / num_boids);

    // 過去の世代になるほど、表示スケールを少しずつ小さくする（消えていくような残像）
    let size_scale = 1.0 - (generation * 0.04);

    // 【進行方向（速度）への回転処理】
    // 速度ベクトルから atan2 を用いてラジアン角度を求める
    let angle = atan2(instance.boid_vel.y, instance.boid_vel.x);
    // 2D回転行列を構築
    let rotation = mat2x2<f32>(
        cos(angle), -sin(angle),
        sin(angle), cos(angle),
    );

    // 1. メッシュの大きさを小さく縮小（0.01倍）し、世代ごとのスケールを掛けて、進行方向へ回転
    var rotated_pos = rotation * (model.position.xy * 0.01 * size_scale);

    // 2. 画面のアスペクト比でX軸を補正（画面が横長でもアスペクト比で割ることで横伸びする歪みを防ぐ）
    rotated_pos.x = rotated_pos.x / render_params.aspect_ratio;

    // 3. 粒子の中心座標（現在地または過去の履歴位置）を足して、最終的なスクリーン上の位置を計算
    let final_pos = rotated_pos + instance.boid_pos;

    // GPUの座標系（Z=0.0, W=1.0）へマッピングして出力
    out.clip_position = vec4f(final_pos, 0.0, 1.0);

    // 【速度に応じたカラーリング】
    // 粒子の移動スピード（ベクトルの長さ）を計算
    let speed = length(instance.boid_vel);
    let min_s = 0.01; // 最低想定速度
    let max_s = 0.03; // 最高想定速度
    // スピードを min_s〜max_s の範囲で 0.0〜1.0 に正規化（クランプ）
    let t = clamp((speed - min_s) / (max_s - min_s), 0.0, 1.0);

    // 遅い時は青色、速い時はピンク色（マゼンタ）にする
    let color_slow = vec3f(0.0, 0.2, 1.0);
    let color_fast = vec3f(1.0, 0.0, 0.8);
    // 線形補間（mix）でスピードに合わせた中間色を作る
    let final_rgb = mix(color_slow, color_fast, t);

    // 【過去の世代ほど不透明度（アルファ）を下げる】
    var trail_alpha = 1.0 - (generation * 0.06);

    // トレイル描画がオフ（use_trails == 0u）かつ、過去の世代（generation >= 1.0）の場合は完全に透明にする
    if (render_params.use_trails == 0u && generation >= 1.0) {
        trail_alpha = 0.0;
    }

    // 最終カラー情報をセットしてフラグメントシェーダーへ送る
    out.color = vec4f(final_rgb, model.color.a * trail_alpha);

    return out;
}

// --- フラグメントシェーダー (fs_main) ---
// ピクセルごとの最終的な色をそのまま出力する
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}

// =========================================================================
// 💻 コンピュートパイプライン用シェーダー (Compute Pipeline - 物理・並列シミュレーション)
// =========================================================================

// --- 粒子（ボイド）のデータ構造 ---
struct Boid {
    position: vec2f, // 2D平面上の位置座標 (x, y)
    velocity: vec2f, // 2D平面上の移動速度 (vx, vy)
}

// --- 物理パラメータのデータ構造 ---
// CPU(Rust)側から Uniform バッファで動的に送られてくる、シミュレーション調整値
struct Params {
    visual_range: f32,     // 仲間を認識できる視野の広さ（これより外の仲間は計算しない）
    protected_range: f32,  // ぶつからないように緊急で避ける「パーソナルスペース」の広さ
    separation_weight: f32,// ルール1：分離（衝突回避）の力の重み
    alignment_weight: f32, // ルール2：整列（群れの平均方向に合わせる）の力の重み
    cohesion_weight: f32,  // ルール3：結合（群れの重心に引き寄せられる）の力の重み
    max_speed: f32,        // 粒子の最大速度の制限値
    min_speed: f32,        // 粒子の最低速度の制限値
}

// --- GPUのストレージバッファと Uniform バッファ ---
// read: 今回の計算の入力元データ（前フレームのデータ）
@group(0) @binding(0) var<storage, read> boids_src: array<Boid>;
// read_write: 今回の計算結果を書き込む出力先データ（次フレームで描画・計算に使われる）
@group(0) @binding(1) var<storage, read_write> boids_dst: array<Boid>;
@group(0) @binding(2) var<uniform> params: Params;

// --- コンピュートシェーダー本体 (cs_main) ---
// GPUの64個のスレッドを1つのグループ（Workgroup）として並列計算する
@compute @workgroup_size(64)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32> // 全スレッドの中で「自分」が何番目のスレッドかを指すID
) {
    let index = global_id.x; // 今回処理する粒子のインデックス

    // ストレージバッファ全体のバイトサイズから、シミュレーション内の総粒子数を計算
    // （16u は Boid 構造体のバイトサイズ：f32が4つ = 4バイト * 4 = 16バイト）
    let num_boids = arrayLength(&boids_src) / 16u;

    // スレッドIDが粒子数を超えていたら何もせず終了（バッファ外アクセス防止）
    if (index >= num_boids) {
        return;
    }

    // 処理対象の粒子のデータをロード
    var boid = boids_src[index];

    // 【残像（歴史）保存のデータシフト処理】
    // トレイル（残像）を描画するために、過去のフレームデータを後ろへ1つずつズラしていく
    // boids_dst[過去の世代] = boids_src[1つ手前の過去の世代]
    for (var g = 15u; g > 0u; g = g - 1u) {
        boids_dst[index + num_boids * g] = boids_src[index + num_boids * (g - 1u)];
    }

    // --- パラメータのロード ---
    let visual_range = params.visual_range;     // 仲間の認識視野
    let protected_range = params.protected_range; // 至近距離（ぶつかる警告範囲）
    
    let separation_weight = params.separation_weight;
    let alignment_weight = params.alignment_weight;
    let cohesion_weight = params.cohesion_weight;
    
    // --- 近傍の仲間から受ける力を計算するための一時変数 ---
    var close_dx = 0.0; // 衝突回避のためのX方向の反発力
    var close_dy = 0.0; // 衝突回避のためのY方向の反発力
    
    var vel_avg = vec2f(0.0, 0.0); // 視野内の仲間の平均速度ベクトル（整列用）
    var pos_avg = vec2f(0.0, 0.0); // 視野内の仲間の平均重心座標（結合用）
    var neighboring_boids = 0.0;   // 視野内に何匹の仲間がいるかのカウンタ

    // 【他のすべての粒子との当たり判定ループ】
    // ※流体シミュレーターを作る際は、ここが「圧力」や「粘性」の計算に置き換わります！
    for (var i = 0u; i < num_boids; i++) {
        if (i == index) { 
            continue; // 自分自身は無視する
        }
        
        let other = boids_src[i];
        
        // 自分と他の粒子との2次元距離を計算
        let d = distance(boid.position, other.position);

        // 🌟 ルール1：分離（Separation）- 近すぎる仲間から離れようとする
        if (d < protected_range) {
            // 自分から相手を遠ざける方向のベクトルを加算していく
            close_dx += boid.position.x - other.position.x;
            close_dy += boid.position.y - other.position.y;
        } 
        // 🌟 ルール2＆3の対象：視野（visual_range）の範囲内にいる場合
        else if (d < visual_range) {
            pos_avg += other.position; // 仲間の位置を合計
            vel_avg += other.velocity; // 仲間の速度を合計
            neighboring_boids += 1.0;  // カウントを増やす
        }
    }

    // 各ルールから生まれる力を合成するための「操舵力（Steering Force）」
    var steering = vec2f(0.0, 0.0);

    // 1. 分離（衝突回避）の力を加算
    steering += vec2f(close_dx, close_dy) * separation_weight;

    // 視野内に仲間がいた場合、整列と結合の力を計算
    if (neighboring_boids > 0.0) {
        // 算術平均を求めて重心位置と平均速度を割り出す
        pos_avg = pos_avg / neighboring_boids; // 視野内の仲間の「集団の真ん中」
        vel_avg = vel_avg / neighboring_boids; // 視野内の仲間の「みんなの進む向き」

        // 🌟 ルール2：結合（Cohesion）- 集団の重心位置に向かって引き寄せられる力
        let cohesion_force = (pos_avg - boid.position) * cohesion_weight;

        // 🌟 ルール3：整列（Alignment）- 集団の平均速度（向き）に自分の速度を合わせようとする力
        let alignment_force = (vel_avg - boid.velocity) * alignment_weight;

        steering += cohesion_force + alignment_force;
    }

    // 計算した操舵力を、現在の速度に滑らかに適用（0.05を掛けることでマイルドに追従させる）
    boid.velocity += steering * 0.05;

    // --- 速度の安全制限（クランプ処理） ---
    let max_speed = params.max_speed;
    let min_speed = params.min_speed;
    let speed = length(boid.velocity);
    
    // スピード制限：速すぎたら最高速度に、遅すぎたら最低速度に調整する
    if (speed > max_speed) {
        boid.velocity = (boid.velocity / speed) * max_speed;
    } else if (speed < min_speed && speed > 0.0) {
        boid.velocity = (boid.velocity / speed) * min_speed;
    }

    // 【位置の更新】
    // 新しい速度に基づいて、現在の位置を進める（オイラー法による積分）
    boid.position += boid.velocity;

    // --- 画面端のワープ（ループ処理） ---
    // 画面外（-1.0 〜 1.0の範囲外）に出たら、反対側から出てくるようにワープさせる
    if (boid.position.x > 1.0) { boid.position.x = -1.0; }
    if (boid.position.x < -1.0) { boid.position.x = 1.0; }
    if (boid.position.y > 1.0) { boid.position.y = -1.0; }
    if (boid.position.y < -1.0) { boid.position.y = 1.0; }

    // 次のフレームで使用するために、今回の結果を出力バッファ（index 0 ＝ 最新世代）に書き込む
    boids_dst[index] = boid;
}
