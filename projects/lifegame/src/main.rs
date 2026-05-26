mod app;
mod state;
mod cell;
mod shape;
mod board;

use app::App;
use winit::event_loop::EventLoop;
use crate::{board::Board, shape::INITIAL_NUM_GRID_PER_ROW};

fn main() {
    env_logger::init();

    let mut app = App::default();

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut app).unwrap();
}
