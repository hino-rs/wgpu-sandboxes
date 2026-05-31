use std::sync::Arc;

use egui::Context as EguiContext;
use egui_winit::State as EguiState;
use web_time::Instant;
use winit::{application::ApplicationHandler, dpi::PhysicalPosition, event::WindowEvent, window::Window};

use crate::{config::{Configs, Processor}, gpu::State, object::{Board, INITIAL_NUM_GRID_PER_ROW}};

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
    configs: Option<Configs>,
    last_processor: Option<Processor>,
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
        self.configs = Some(Configs::default());
        self.last_processor = Some(Processor::CPU);

        let board = self.board.as_ref().unwrap();
        self.state.as_mut().unwrap().upload_board(&board.current_squares);
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
                    Some(configs)
                ) = (
                    &mut self.state,
                    &mut self.board,
                    &mut self.window,
                    &mut self.egui_state,
                    &mut self.last_update_time,
                    &mut self.configs,
                ) {
                    let now = Instant::now();
                    let elapsed = now.duration_since(*last_update);

                    // プロセッサ切り替えの検知と同期
                    if Some(configs.sys.processor) != self.last_processor {
                        match configs.sys.processor {
                            Processor::CPU => {
                                // GPUからCPUへ：シミュレーション結果をダウンロード
                                state.download_board(&mut board.current_squares);
                            }
                            Processor::GPU => {
                                // CPUからGPUへ：シミュレーション開始のためにアップロード
                                state.upload_board(&board.current_squares);
                            }
                        }
                        self.last_processor = Some(configs.sys.processor);
                    }

                    if !board.pause {
                        if elapsed.as_millis() >= board.delay as u128 {
                            match configs.sys.processor {
                                Processor::CPU => board.update(),
                                Processor::GPU => state.update_compute(board.current_squares.len()),
                            }
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
                            let nx = (pos.x / width) * 2.0 - 1.0;
                            let ny = 1.0 - (pos.y / height) * 2.0;
                            let cell_pitch = 2.0 / (board.num_grid_per_row - 1) as f64;
                            let gx = ((nx + 1.0) / cell_pitch).round() as isize;
                            let gy = ((ny + 1.0) / cell_pitch).round() as isize;

                            let brush_radius = configs.sim.brush_radius;
                            
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
                            
                            // 送信
                            match configs.sys.processor {
                                Processor::CPU => {
                                    // CPUモード：更新されたデータをインスタンスバッファへ転送
                                    state.update_instances(
                                        &board.current_squares,
                                        board.num_grid_per_row,
                                    );
                                }
                                Processor::GPU => {
                                    // GPUモード：ブラシの uniform 情報を更新
                                    state.update_brush(
                                        true,
                                        gx as f32,
                                        gy as f32,
                                        brush_radius as f32,
                                        0.2,
                                    );
                                }
                            }
                        }
                    } else {
                        if configs.sys.processor == Processor::GPU {
                            state.update_brush(false, 0.0, 0.0, 0.0, 0.0);
                        }
                    }


                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
                        ui.heading("Fake Water Control Panel");

                        ui.label("Processor");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut configs.sys.processor, crate::config::Processor::CPU, "CPU");
                            ui.selectable_value(&mut configs.sys.processor, crate::config::Processor::GPU, "GPU");
                        });
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

                    if configs.sys.processor == Processor::CPU {
                        state.update_instances(
                            &board.current_squares,
                            board.num_grid_per_row,
                        );
                    }
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
            configs: Some(Configs::default()),
            last_processor: Some(Processor::CPU),
        }
    }
}

impl App {

}
