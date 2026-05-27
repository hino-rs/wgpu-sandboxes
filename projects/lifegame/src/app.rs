use std::sync::Arc;

use egui::Context as EguiContext;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use egui_winit::State as EguiState;
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};
use web_time::Instant;

use crate::{board::Board, shape::INITIAL_NUM_GRID_PER_ROW, state::State};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    pub board: Option<Board>,
    egui_ctx: EguiContext,
    egui_state: Option<EguiState>,
    current_tab: ConfigTab,
    last_update_time: Option<Instant>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

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
                if let (Some(state), Some(board), Some(window), Some(egui_state), Some(last_update)) = (
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
                        if board.next_tick {
                            board.update();
                            *last_update = now;
                        }
                    }

                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
                        ui.heading("LifeGame Simulator Control Panel");

                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.current_tab,
                                ConfigTab::Simulation,
                                "🎮 Sim",
                            );
                            ui.selectable_value(
                                &mut self.current_tab,
                                ConfigTab::Graphics,
                                "🖼 Style",
                            );
                            ui.selectable_value(
                                &mut self.current_tab,
                                ConfigTab::Stats,
                                "📊 Stats",
                            );
                        });

                        ui.separator();

                        match self.current_tab {
                            ConfigTab::Simulation => Self::draw_simulation_ui(ui, board, state),
                            ConfigTab::Graphics => Self::draw_graphics_ui(ui, board, state),
                            ConfigTab::Stats => Self::draw_stats_ui(ui, board),
                        }
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
                        board.cells(),
                        board.num_grid_per_row,
                        board.gap_size,
                        board.alive_cell_color,
                        board.dead_cell_color,
                    );
                    state.render(&paint_jobs, &screen_descripter, board.bg_color);
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
        }
    }
}

impl App {
    fn draw_stats_ui(ui: &mut egui::Ui, board: &Board) {
        let (alive, dead) = board.alive_dead_count;

        ui.heading("Current Stats");
        ui.label(format!("Board Length: {}", board.num_grid_per_row));
        ui.label(format!("Cell Count:   {}", board.grid_size));
        ui.label(format!("Alive: {alive}"));
        ui.label(format!("Dead:  {dead}"));

        Self::plot_record_of_cells(ui, board);
    }

    fn draw_board_resize_ui(ui: &mut egui::Ui, board: &mut Board, state: &mut State) {
        let ratio_id = ui.id().with("board_resize_ratio");
        let mut ratio = ui
            .ctx()
            .data(|map| map.get_temp::<u8>(ratio_id).unwrap_or(1));

        ui.label("Resize Control Ratio");
        if ui.add(egui::Slider::new(&mut ratio, 1..=u8::MAX)).changed() {
            ui.ctx().data_mut(|map| map.insert_temp(ratio_id, ratio));
        }

        if ui
            .toggle_value(&mut false, format!("+ Increase Size x{ratio}"))
            .clicked()
        {
            board.expand(state, ratio);
        }
        if ui
            .toggle_value(&mut false, format!("- Decrease Size x{ratio}"))
            .clicked()
        {
            board.shrink(state, ratio);
        }
    }

    fn draw_simulation_ui(ui: &mut egui::Ui, board: &mut Board, state: &mut State) {
        // 一時停止
        ui.toggle_value(&mut board.pause, "Pause");

        // クロックを1つ進める
        if board.pause {
            ui.toggle_value(&mut board.next_tick, "Next Tick");
        }

        // 遅延
        ui.label("Delay");
        ui.add(
            egui::Slider::new(&mut board.delay, 0..=1000)
                .custom_formatter(|val, _| format!("{val}msec")),
        );

        // ランダムの確率
        ui.label("Random True Ratio");
        ui.add(egui::Slider::new(&mut board.random_ratio, 0.00..=1.00))
            .on_hover_text("This setting affects the Alive rate during initial generation and the probability for randomly setting entities to Alive/Dead.");

        // 再シャッフル
        if ui.toggle_value(&mut false, "Reshuffle").clicked() {
            board.reshuffle();
        }

        // 盤面クリア
        if ui.toggle_value(&mut false, "Clear").clicked() {
            board.clear();
        }

        // ランダムにAliveにさせる
        if ui.toggle_value(&mut false, "Randomly make Alive").clicked() {
            board.randomly_make_alive();
        }

        // ランダムにDeadにさせる
        if ui.toggle_value(&mut false, "Randomly make Dead").clicked() {
            board.randomly_make_dead();
        }

        Self::draw_board_resize_ui(ui, board, state);
    }

    fn draw_graphics_ui(ui: &mut egui::Ui, board: &mut Board, state: &mut State) {
        ui.label("Background Color");
        ui.color_edit_button_rgb(&mut board.bg_color);

        ui.label("Alive Cell Color");
        ui.color_edit_button_rgb(&mut board.alive_cell_color);

        ui.label("Dead Cell Color");
        ui.color_edit_button_rgb(&mut board.dead_cell_color);

        ui.label("Grid Gap Size");
        if ui
            .add(egui::Slider::new(&mut board.gap_size, 0.0..=1.0))
            .changed()
        {
            state.update_instances(
                &board.current,
                board.num_grid_per_row,
                board.gap_size,
                board.alive_cell_color,
                board.dead_cell_color,
            );
        };
    }

    fn plot_record_of_cells(ui: &mut egui::Ui, board: &Board) {
        let record = board.record.clone();

        let mut alive_record = Vec::with_capacity(100);
        let mut dead_record = Vec::with_capacity(100);

        for r in record {
            alive_record.push(r.alive_count);
            dead_record.push(r.dead_count);
        }

        let alive_points: PlotPoints = alive_record
            .iter()
            .enumerate()
            .map(|(i, &v)| [i as f64, v as f64])
            .collect();
        let alive_line = Line::new("Alive", alive_points).color(egui::Color32::from_rgb(0, 255, 0));

        let dead_points: PlotPoints = dead_record
            .iter()
            .enumerate()
            .map(|(i, &v)| [i as f64, v as f64])
            .collect();
        let dead_line = Line::new("Dead", dead_points).color(egui::Color32::from_rgb(255, 0, 0));

        Plot::new("Alive & Dead Record")
            .view_aspect(2.0)
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(alive_line);
                plot_ui.line(dead_line)
            });
    }
}
