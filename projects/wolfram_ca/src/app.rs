use std::sync::Arc;

use egui::Context as EguiContext;
use egui_winit::State as EguiState;
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};
use web_time::Instant;

use crate::{
    ca::Ca,
    state::{INITIAL_NUM_OF_BITS, State},
};

pub const INITIAL_RULE: u8 = 30;

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    ca: Option<Ca>,
    egui_ctx: EguiContext,
    egui_state: Option<EguiState>,
    last_update_time: Option<Instant>,
    delay: u8,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Wolfram CA"))
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

        let mut cells = vec![0; INITIAL_NUM_OF_BITS as usize];
        cells[INITIAL_NUM_OF_BITS as usize / 2] = 1;

        let ca = Ca {
            rule: INITIAL_RULE,
            num_of_bits: INITIAL_NUM_OF_BITS,
            cells,
            pause: false,
            color_of_1: [1.0, 1.0, 1.0],
            color_of_0: [0.0, 0.0, 0.0],
            circulation: false,
            stay: false,
        };

        self.window = Some(window);
        self.state = Some(state);
        self.ca = Some(ca);
        self.egui_state = Some(egui_state);
        self.last_update_time = Some(Instant::now());
        self.delay = 0;
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

            WindowEvent::RedrawRequested => {
                if let (Some(window), Some(state), Some(ca), Some(egui_state), Some(last_update_time)) = (
                    &mut self.window,
                    &mut self.state,
                    &mut self.ca,
                    &mut self.egui_state,
                    &mut self.last_update_time,
                ) {
                    let now = Instant::now();
                    let elapsed = now.duration_since(*last_update_time);
                    if elapsed.as_millis() >= self.delay as u128 {
                        ca.append_next();
                        *last_update_time = now;
                    }
                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Config").show(&self.egui_ctx, |ui| {
                        ui.heading("Wolfram Cellular Automata");

                        ui.label("Rule");
                        if ui
                            .add(egui::Slider::new(&mut ca.rule, 0..=u8::MAX))
                            .changed()
                        {
                            ca.change_bits();
                        }

                        if ui.button("Increase Rule by 1").clicked() {
                            if ca.rule < 255 {
                                ca.rule += 1;
                                ca.change_bits();
                            }
                        }

                        if ui.button("Decrease Rule by 1").clicked() {
                            if ca.rule > 0 {
                                ca.rule -= 1;
                                ca.change_bits();
                            }
                        }

                        ui.checkbox(&mut ca.circulation, "Circulation");
                        ui.checkbox(&mut ca.stay, "Stay");

                        ui.label("Delay");
                        ui.add(
                            egui::Slider::new(&mut self.delay, 0..=u8::MAX)
                                .custom_formatter(|val, _| format!("{val}msec")),
                        );

                        ui.label("Num Of Bits");
                        if ui
                            .add(egui::Slider::new(&mut ca.num_of_bits, 1..=4096))
                            .changed()
                        {
                            ca.change_bits();
                        };

                        ui.toggle_value(&mut ca.pause, "Pause");

                        ui.label("Color Of `1` Cells");
                        ui.color_edit_button_rgb(&mut ca.color_of_1);

                        ui.label("Color Of `0` Cells");
                        ui.color_edit_button_rgb(&mut ca.color_of_0);

                        ui.label("Background Color");
                        ui.color_edit_button_rgb(&mut state.bg_color);
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

                    state.update_instances(&ca.cells, ca.num_of_bits, ca.color_of_1, ca.color_of_0);
                    state.render(&paint_jobs, &screen_descripter);

                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

impl App {}
