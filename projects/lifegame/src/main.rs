mod app;
mod board;
mod shape;
mod state;
mod utils;

use app::App;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let mut app = App::default();

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run() {
    use crate::state::State;
    use std::sync::Arc;
    use winit::window::Window;

    // ブラウザのデベロッパーコンソールにRustのパニックログをきれいに出力する
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    let event_loop = EventLoop::new().unwrap();

    // Wasm環境用の非同期コンテキストを立ち上げる
    wasm_bindgen_futures::spawn_local(async move {
        // 先行して Window を生成
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu lifegame"))
                .unwrap(),
        );

        // Canvas要素をHTMLの<body>に追加し、画面いっぱいに広げる
        {
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().unwrap();
            let web_window = web_sys::window().unwrap();
            let document = web_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(&canvas).unwrap();
        }

        // スレッドをブロックせず、awaitでGPU初期化を待つ
        let state = State::new(Arc::clone(&window)).await;

        // 先行生成した Window/State を使って App を起動
        let mut app = App::with_precreated(window, state);
        event_loop.run_app(&mut app).unwrap();
    });
}
