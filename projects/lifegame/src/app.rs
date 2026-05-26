use std::sync::Arc;

use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};
use egui::Context as EguiContext;
use egui_winit::State as EguiState;

use crate::{board::Board, shape::INITIAL_NUM_GRID_PER_ROW, state::State};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    pub board: Option<Board>,
    egui_ctx: EguiContext,
    egui_state: Option<EguiState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu triangle"))
                .unwrap(),
        );

        let state = pollster::block_on(State::new(Arc::clone(&window)));
        
        let egui_state = EguiState::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );
    
        self.window = Some(window);
        self.state = Some(state);
        self.board = Some(Board::new(INITIAL_NUM_GRID_PER_ROW));
        self.egui_state = Some(egui_state);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(egui_state) = &mut self.egui_state {
            let response = egui_state.on_window_event(self.window.as_ref().unwrap(), &event);

            if response.consumed {
                return;
            }
        }

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
                if let (Some(state), Some(board), Some(window), Some(egui_state)) = (&mut self.state, &mut self.board, &mut self.window, &mut self.egui_state) {
                    board.update();
                    
                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
                        ui.label("LifeGame Simulator Control Panel");

                        if ui.button("Reset").clicked() {

                        }
                    });

                    let egui_output = self.egui_ctx.end_pass();
                    egui_state.handle_platform_output(window, egui_output.platform_output);
                    
                    for (id, image_delta) in &egui_output.textures_delta.set {
                        state.egui_renderer.update_texture(
                            &state.device, 
                            &state.queue, 
                            *id, 
                            image_delta
                        );
                    }

                    for id in &egui_output.textures_delta.free {
                        state.egui_renderer.free_texture(id);
                    }

                    let paint_jobs = self.egui_ctx.tessellate(egui_output.shapes, egui_output.pixels_per_point);

                    let screen_descripter = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [state.config.width, state.config.height],
                        pixels_per_point: egui_output.pixels_per_point,
                    };
                    
                    state.update_instances(board.cells(), board.num_grid_per_row, 0.0); // GAP
                    state.render(&paint_jobs, &screen_descripter);
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
