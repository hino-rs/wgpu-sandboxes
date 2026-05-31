mod app;
mod gpu;
mod object;
mod utils;
mod config;

use app::App;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let mut app = App::default();

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut app).unwrap();
}