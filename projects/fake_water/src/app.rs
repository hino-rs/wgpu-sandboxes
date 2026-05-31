use std::sync::Arc;

use egui::Context as EguiContext;
use egui_winit::State as EguiState;
use web_time::Instant;
use winit::{application::ApplicationHandler, dpi::PhysicalPosition, event::WindowEvent, window::Window};

use crate::{gpu::State, object::{Board, INITIAL_NUM_GRID_PER_ROW}};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    pub board: Option<Board>,
    egui_ctx: EguiContext,
    egui_state: Option<EguiState>,
    current_tab: ConfigTab,
    last_update_time: Option<Instant>,
    cursor_pos: Option<winit::dpi::PhysicalPosition<f64>>,
    mouse_pressed: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("LifeGame"))
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
        self.current_tab = ConfigTab::default();
        self.last_update_time = Some(Instant::now());
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
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            // WindowEvent::KeyboardInput {
            //     device_id,
            //     event,
            //     is_synthetic,
            // } => {}
            WindowEvent::RedrawRequested => {
                if let (
                    Some(state),
                    Some(board),
                    Some(window),
                    Some(egui_state),
                    Some(last_update),
                ) = (
                    &mut self.state,
                    &mut self.board,
                    &mut self.window,
                    &mut self.egui_state,
                    &mut self.last_update_time,
                ) {
                    let now = Instant::now();
                    let elapsed = now.duration_since(*last_update);
                    if !board.pause {
                        if elapsed.as_millis() >= board.delay as u128 {
                            board.update();
                            *last_update = now;
                        }
                    } else {
                        if board.next_step {
                            board.update();
                            *last_update = now;
                        }
                    }

                    if self.mouse_pressed {
                        if let Some(pos) = &self.cursor_pos {
                            let size = window.inner_size();
                            let width = size.width as f64;
                            let height = size.height as f64;

                            // 座標をwgpuで扱いやすいように変換
                            let nx = (pos.x / width) * 2.0 - 1.0;
                            let ny = 1.0 - (pos.y / height) * 2.0;

                            // グリッド座標に変換
                            let cell_pitch = 1.6 / (board.num_grid_per_row - 1) as f64;
                            let gx = ((nx + 0.8) / cell_pitch).round() as isize;
                            let gy = ((ny + 0.8) / cell_pitch).round() as isize;

                            // 円形ブラシで水を付与する
                            let brush_radius = 4;
                            for dy in -brush_radius..=brush_radius {
                                for dx in -brush_radius..=brush_radius {
                                    // 円の内側かチェック
                                    if dx * dx + dy * dy <= brush_radius * brush_radius {
                                        let tx = gx + dx;
                                        let ty = gy + dy;

                                        // グリッドの範囲内かチェック
                                        if tx >= 0 && tx < board.num_grid_per_row as isize
                                            && ty >= 0 && ty < board.num_grid_per_row as isize 
                                        {
                                            let idx = ty as usize * board.num_grid_per_row + tx as usize;
                                            
                                            // puddleに水を加える
                                            board.current_squares[idx].puddle = f32::min(
                                                1.0, 
                                                board.current_squares[idx].puddle + 0.2
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }


                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
                        ui.heading("Fake Water Control Panel");
                    });

                    let egui_output = self.egui_ctx.end_pass();
                    egui_state.handle_platform_output(window, egui_output.platform_output);

                    for (id, image_delta) in &egui_output.textures_delta.set {
                        state.egui_renderer.update_texture(
                            &state.device,
                            &state.queue,
                            *id,
                            image_delta,
                        );
                    }

                    for id in &egui_output.textures_delta.free {
                        state.egui_renderer.free_texture(id);
                    }

                    let paint_jobs = self
                        .egui_ctx
                        .tessellate(egui_output.shapes, egui_output.pixels_per_point);

                    let screen_descripter = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [state.config.width, state.config.height],
                        pixels_per_point: egui_output.pixels_per_point,
                    };

                    state.update_instances(
                        &board.current_squares,
                        board.num_grid_per_row,
                    );
                    state.render(&paint_jobs, &screen_descripter, board.bg_color);
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = Some(position);
                // let window_size = &self.state.as_ref().unwrap().config;
                
                // let width = window_size.width;
                // let height = window_size.height;

                // let nx = (position.x / width as f64) * 2.0 - 1.0;
                // let ny = 1.0 - (position.y / height as f64) * 2.0;

                // self.cursor_pos = Some(PhysicalPosition {
                //     x: nx,
                //     y: ny,
                // });
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    self.mouse_pressed = state.is_pressed();
                }
            }

            _ => {}
        }
    }
}

#[derive(PartialEq, Clone, Copy, Default)]
enum ConfigTab {
    #[default]
    Simulation,
    Graphics,
    Stats,
}

impl App {
    pub fn with_precreated(window: Arc<Window>, state: State) -> Self {
        let egui_ctx = EguiContext::default();
        let egui_state = EguiState::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        Self {
            window: Some(window),
            state: Some(state),
            board: Some(Board::new(INITIAL_NUM_GRID_PER_ROW)),
            egui_state: Some(egui_state),
            egui_ctx,
            current_tab: ConfigTab::default(),
            last_update_time: Some(Instant::now()),
            cursor_pos: Some(PhysicalPosition::default()),
            mouse_pressed: false,
        }
    }
}

impl App {

}
