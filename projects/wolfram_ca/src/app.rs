use std::sync::Arc;

use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

use crate::{ca::Ca, state::{INITIAL_NUM_OF_BITS, State}};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state:  Option<State>,
    ca:     Option<Ca>
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Wolfram CA")).unwrap()
        );

        let state = pollster::block_on(State::new(Arc::clone(&window)));

        let mut cells = vec![0; INITIAL_NUM_OF_BITS as usize];
        cells[INITIAL_NUM_OF_BITS as usize / 2] = 1;

        let ca = Ca { 
            num_of_bits: INITIAL_NUM_OF_BITS, 
            cells,
        };
        
        self.window = Some(window);
        self.state =  Some(state);
        self.ca = Some(ca)
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    )
    {
        match event {
            WindowEvent::Resized(physical_size) => {
                if let Some(state) = &mut self.state {
                    state.resize(physical_size);
                }
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let (
                    Some(window), 
                    Some(state),
                    Some(ca),
                ) = (
                    &mut self.window, 
                    &mut self.state,
                    &mut self.ca,
                ) {
                    window.request_redraw();
                    ca.append_next();
                    state.render();
                    state.update_instances(&ca.cells, ca.num_of_bits);
                }
            }
            
            _ => {}
        }
    }    
}

impl App {

}