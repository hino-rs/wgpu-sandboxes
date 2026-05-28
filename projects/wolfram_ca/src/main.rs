mod app;
mod state;
mod shader;
mod ca;

fn main() {
    env_logger::init();

    let mut app = app::App::default();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.run_app(&mut app).unwrap();
}
