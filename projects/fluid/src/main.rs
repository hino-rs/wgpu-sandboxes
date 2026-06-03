mod app;
mod fluid;
mod gpu;
mod gui;
mod renderer;
mod types;

use app::App;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let mut app = App::default();

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut app).unwrap();
}
