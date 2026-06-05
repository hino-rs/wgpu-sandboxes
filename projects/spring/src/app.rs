use std::sync::Arc;

use egui::Context as EguiContext;
use egui_winit::State as EguiState;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{MouseButton, TouchPhase, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{
    core::Object,
    state::{State, Uniform},
};

#[derive(Default, PartialEq)]
enum Tab {
    #[default]
    Control,
    Color,
}

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    egui_ctx: EguiContext,
    egui_state: Option<EguiState>,

    cursor_pos: PhysicalPosition<f32>,
    is_dragging: bool,
    pub object: Object,
    pub dt: f32,
    pub c: f32, // 抵抗
    pub k: f32, // 硬さ
    pub g: f32, // 重力

    current_tab: Tab,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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

        let egui_ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "font_ja".to_owned(),
            Arc::from(egui::FontData::from_owned(
                include_bytes!("../../../common/NotoSansJP-VariableFont_wght.ttf").to_vec(),
            )),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "font_ja".to_owned());
        egui_ctx.set_fonts(fonts);

        self.state = Some(state);
        self.window = Some(window);
        self.egui_state = Some(egui_state);
        self.egui_ctx = egui_ctx;
        self.object = Object::default();
        self.dt = 0.05;
        self.c = 0.5;
        self.k = 4.0;
        self.g = 1.0;
        self.current_tab = Tab::Control;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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

            WindowEvent::Touch(touch) => {
                let phase = touch.phase;

                if phase == TouchPhase::Ended {
                    self.is_dragging = false;
                    return;
                }

                let position = touch.location;

                let window_size = &self.state.as_ref().unwrap().config;
                let width = window_size.width;
                let height = window_size.height;

                let nx = ((position.x / width as f64) * 2.0 - 1.0) as f32;
                let ny = (1.0 - (position.y / height as f64) * 2.0) as f32;

                self.cursor_pos = PhysicalPosition::new(nx, ny);

                if phase == TouchPhase::Started {
                    self.is_dragging = true;
                    self.object.update_pos(nx, ny);
                }

                if phase == TouchPhase::Moved {
                    if self.is_dragging {
                        self.object.update_pos(self.cursor_pos.x, self.cursor_pos.y);
                    }
                }
            }

            WindowEvent::MouseInput { button, state, .. } => {
                if button == MouseButton::Left {
                    if state.is_pressed() && self.egui_ctx.egui_wants_pointer_input() {
                        return;
                    }
                    self.is_dragging = state.is_pressed();

                    if self.is_dragging {
                        self.object.update_pos(self.cursor_pos.x, self.cursor_pos.y);
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let window_size = &self.state.as_ref().unwrap().config;

                let width = window_size.width;
                let height = window_size.height;

                let nx = ((position.x / width as f64) * 2.0 - 1.0) as f32;
                let ny = (1.0 - (position.y / height as f64) * 2.0) as f32;

                if self.is_dragging {
                    self.object.update_pos(nx, ny);
                }

                self.cursor_pos = PhysicalPosition { x: nx, y: ny };
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.update();

                if let (Some(state), Some(window), Some(egui_state)) =
                    (&mut self.state, &self.window, &mut self.egui_state)
                {
                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Config").show(&self.egui_ctx, |ui| {
                        ui.heading("Spring Simulator");

                        ui.separator();
                        ui.heading("State");
                        ui.label(format!("座標(x, y): ({:>6.2}, {:>6.2})", self.object.x, self.object.y));
                        ui.label(format!("速度(x, y): ({:>6.2}, {:>6.2})", self.object.ax.abs(), self.object.ay.abs()));
                        ui.label(format!("加速(x, y): ({:>6.2}, {:>6.2})", self.object.vx.abs(), self.object.vy.abs()));

                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.current_tab, Tab::Control, "操作とパラメータ調整");
                            ui.selectable_value(&mut self.current_tab, Tab::Color, "色");
                        });

                        ui.separator();

                        match self.current_tab {
                            Tab::Control => {
                                ui.heading("Control");
                                if ui.button("リセット").clicked() {
                                    self.object.reset();
                                };

                                ui.separator();

                                ui.heading("Parameters");
                                ui.add(egui::Slider::new(&mut self.g, 0.0..=10.0).text("重力"));
                                ui.add(egui::Slider::new(&mut self.k, 0.0..=10.0).text("バネの硬さ"));
                                ui.add(egui::Slider::new(&mut self.c, 0.0..=10.0).text("抵抗"));

                                if ui
                                    .add(
                                        egui::Slider::new(&mut self.dt, 0.0001..=0.5)
                                            .text("オイラー法 刻み"),
                                    )
                                    .changed()
                                {
                                    self.object.reset();
                                }
                            }
                            Tab::Color => {
                                ui.heading("Color");
                                ui.label("背景");
                                ui.color_edit_button_rgb(&mut state.bg_color);
                                ui.label("おもり");
                                ui.color_edit_button_rgb(&mut self.object.weight_color);
                                ui.label("紐");
                                ui.color_edit_button_rgb(&mut self.object.spring_color);
                            }
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

                    state.render(&paint_jobs, &screen_descripter);
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

impl App {
    fn update(&mut self) {
        if let Some(gpu) = &self.state {
            if !self.is_dragging {
                self.object.calc(self.dt, self.g, self.k, self.c);
            }

            let uniform_data = Uniform {
                x: self.object.x,
                y: self.object.y,
                _p1: [0.0, 0.0],

                spring_color: self.object.spring_color,
                _p2: 0.0,
                
                weight_color: self.object.weight_color,
                _p3: 0.0,
            };

            gpu.queue
                .write_buffer(&gpu.uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));
        }
    }
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
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "font_ja".to_owned(),
            Arc::from(egui::FontData::from_owned(
                include_bytes!("../../../common/NotoSansJP-VariableFont_wght.ttf").to_vec(),
            )),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "font_ja".to_owned());
        egui_ctx.set_fonts(fonts);

        Self {
            window: Some(window),
            state: Some(state),
            egui_state: Some(egui_state),
            egui_ctx,
            cursor_pos: PhysicalPosition::default(),
            is_dragging: false,
            object: Object::default(),
            dt: 0.05,
            c: 0.5,
            k: 4.0,
            g: 1.0,
            current_tab: Tab::Control,
        }
    }
}
