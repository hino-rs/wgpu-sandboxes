use std::sync::Arc;

use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

use crate::{board::Board, shape::INITIAL_NUM_GRID_PER_ROW, state::State};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    pub board: Option<Board>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu triangle"))
                .unwrap(),
        );

        let state = pollster::block_on(State::new(Arc::clone(&window)));
        self.window = Some(window);
        self.state = Some(state);
        self.board = Some(Board::new(INITIAL_NUM_GRID_PER_ROW));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(physical_size) => {
                if let Some(state) = &mut self.state {
                    state.resize(physical_size);
                } 
            },

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            // WindowEvent::KeyboardInput {
            //     device_id,
            //     event,
            //     is_synthetic,
            // } => {}

            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(board)) = (&mut self.state, &mut self.board) {
                    board.update();
                    state.update_instances(board.cells(), board.num_grid_per_row, 0.0); // GAP
                    state.render();
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            // WindowEvent::CursorMoved { position, .. } => {}

            // WindowEvent::MouseInput {
            //     device_id,
            //     state,
            //     button,
            // } => {}

            _ => {}
        }
    }
}
