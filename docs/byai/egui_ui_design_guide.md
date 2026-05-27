# 🎨 egui UI構築とレイアウト整理の極意（Rust GUI設計ガイド）

egui は「書いたコードが即座に画面に描画される（Immediate Mode GUI）」という非常に強力で直感的なフレームワークですが、機能が増えるにつれて **UI描画コードが巨大な入れ子（ネスト）になり、視認性と保守性が急激に低下する** という特有の課題を持っています。

このドキュメントでは、egui のUIコードを美しく整理し、拡張性を維持するための設計テクニックとコツを解説します。

---

## 🧭 1. UIコードをコンポーネント化（関数分割）する

すべてのUIコントロールを1つの巨大なクロージャ（`egui::Window::show` など）の中に書くのは避けましょう。項目ごとに **「Ui を受け取る小さな関数」** へ切り出します。

### パターンA：状態を読み取るだけのUI（グラフやステータス表示など）
```rust
impl App {
    // 状態を変更しないUIは `&self` で受け取る
    fn draw_stats_ui(&self, ui: &mut egui::Ui, board: &Board) {
        ui.heading("Current Stats");
        let (alive, dead) = board.alive_dead_count;
        ui.label(format!("Board Length: {}", board.num_grid_per_row));
        ui.label(format!("Alive: {alive}"));
        ui.label(format!("Dead:  {dead}"));
    }
}
```

### パターンB：状態を変更するUI（操作パネルなど）
```rust
impl App {
    // 操作を伴うUIは `&mut self` (または `&mut Board`) を受け取る
    fn draw_controls_ui(&mut self, ui: &mut egui::Ui, board: &mut Board) {
        ui.heading("Controls");
        if ui.button("Reset").clicked() {
            board.reshuffle();
        }
    }
}
```

### 呼び出し側のコード（劇的にすっきりします）
```rust
egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
    // 関数に分割したことで、メインループの見通しが良くなります
    self.draw_stats_ui(ui, board);
    ui.separator();
    self.draw_controls_ui(ui, board);
});
```

---

## 🗂️ 2. タブ（Tab）による省スペース化とカテゴリ分け

項目が膨大になった場合、一番効果的なのは **「画面上部でタブを切り替えてUIを丸ごと差し替える」** 設計です。

### 実装手順

1. タブを表す `enum` を定義します。
```rust
#[derive(PartialEq, Clone, Copy)]
enum ConfigTab {
    Simulation,
    Graphics,
    Stats,
}
```
2. `App` （または `Board`）に、現在選択されているタブの変数を保持します（初期値は `ConfigTab::Simulation` など）。

3. UI構築時に、上部にセレクター（横並びのボタン）を配置し、選択されたタブに応じて描画する関数を切り替えます。

```rust
// app.rs などの UI 構築処理
egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
    // 1. 横並びのタブセレクターを配置
    ui.horizontal(|ui| {
        ui.selectable_value(&mut self.current_tab, ConfigTab::Simulation, "🎮 Sim");
        ui.selectable_value(&mut self.current_tab, ConfigTab::Graphics, "🎨 Style");
        ui.selectable_value(&mut self.current_tab, ConfigTab::Stats, "📊 Data");
    });
    
    ui.separator();

    // 2. 選択されたタブに応じてコンポーネントを出し分け
    match self.current_tab {
        ConfigTab::Simulation => self.draw_simulation_tab(ui, board),
        ConfigTab::Graphics => self.draw_graphics_tab(ui, board),
        ConfigTab::Stats => self.draw_stats_tab(ui, board),
    }
});
```

---

## 🧱 3. 視認性を向上させる egui のレイアウトウィジェット

項目を整理し、ユーザーが直感的に操作できるようにするための便利なレイアウト手法です。

### ① `CollapsingHeader` （折りたたみメニュー）
めったに触らない詳細設定や、色の細かな調整などは、折りたたみの中に隠しておきます。
```rust
egui::CollapsingHeader::new("🛠️ Advanced Settings")
    .default_open(false) // 最初は閉じさせておく
    .show(ui, |ui| {
        ui.add(egui::Slider::new(&mut board.random_ratio, 0.0..=1.0));
        // ... その他の詳細設定
    });
```

### ② `egui::Grid` （格子状整列）
ラベルとスライダーの文字数が異なると、縦のラインがガタガタになって見栄えが悪くなります。`Grid` を使うと、エクセルやテーブルのように綺麗に縦列を揃えられます。
```rust
egui::Grid::new("color_grid")
    .num_columns(2)
    .spacing([40.0, 4.0]) // 列間, 行間
    .striped(true)        // 1行おきに背景色をグレーにして見やすくする
    .show(ui, |ui| {
        ui.label("Alive Color:");
        ui.color_edit_button_rgb(&mut board.alive_color);
        ui.end_row(); // 次の行へ

        ui.label("Dead Color:");
        ui.color_edit_button_rgb(&mut board.dead_color);
        ui.end_row();
    });
```

### ③ `ui.group` と `ui.scope` （グループ化）
関連する設定を薄い枠線で囲むことで、視覚的な「まとまり」を作ります。
```rust
ui.group(|ui| {
    ui.label("Board Size Controls");
    if ui.button("Increase").clicked() { ... }
    if ui.button("Decrease").clicked() { ... }
});
```

---

## 🚀 4. GUI崩れ・はみ出しを防ぐベストプラクティス

項目が縦に長くなりすぎると、画面外にはみ出して操作できなくなることがあります。これを防ぐための防衛策です。

### 🛡️ `ScrollArea` の適用
設定ウインドウ全体（または特定のグループ）を `ScrollArea` で囲むと、ウィンドウサイズが小さくなった場合に自動でスクロールバーが現れます。**UI項目が多い場合は必須のテクニックです。**

```rust
egui::Window::new("Configs")
    .default_height(400.0) // デフォルトの高さ
    .show(&self.egui_ctx, |ui| {
        // 縦方向のスクロールを有効にする
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.draw_all_heavy_ui(ui, board);
        });
    });
```

---

## 💡 まとめ：綺麗な UI を構築するための黄金ルール

1. **画面の縦サイズが足りなくなる前に `ScrollArea` を入れる**。
2. **関連するコントロールは `ui.group` や `CollapsingHeader` にまとめる**。
3. **設定のジャンルが3つ以上になったら `selectable_value` による「タブ切り替え」を検討する**。
4. **1つの関数は最大でも 50〜80行 程度に抑え、それ以上はヘルパー関数に分割する**。
