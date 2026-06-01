mod app;
mod gpu;
mod boids;

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
    use crate::gpu::State;
    use std::sync::Arc;
    use winit::window::Window;

    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    let event_loop = EventLoop::new().unwrap();

    wasm_bindgen_futures::spawn_local(async move {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgp Boids simlator"))
                .unwrap(),
        );

        {
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().unwrap();
            let web_window = web_sys::window().unwrap();
            let document = web_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(&canvas).unwrap();
        }

        let state = State::new(Arc::clone(&window)).await;

        let mut app = App::with_precreated(window, state);
        event_loop.run_app(&mut app).unwrap();
    });
}
