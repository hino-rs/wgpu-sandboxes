use std::sync::Arc;

use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop, window::{Window, WindowId}};

use crate::state::State;

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu triangle"))
                .unwrap()
        );

        let state = pollster::block_on(State::new(Arc::clone(&window)));
        self.window = Some(window);
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.render();
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(state) = &mut self.state {
                    state.cursor_pos = (position.x as f32, position.y as f32);
                }
            }

            WindowEvent::MouseInput { state: button_state, .. } => {
                if let Some(state) = &mut self.state {
                    state.handle_mouse_click(button_state);
                }
            }
            
            _ => {}
        }
    }
}
