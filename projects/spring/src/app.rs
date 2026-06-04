use std::sync::Arc;

use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop, window::{Window, WindowId}};
use egui::Context as EguiContext;
use egui_winit::State as EguiState;

use crate::state::State;

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    egui_ctx: EguiContext,
    egui_state: Option<EguiState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wgpu triangle"))
                .unwrap()
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
        fonts.families.entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "font_ja".to_owned());
        egui_ctx.set_fonts(fonts);
        
        self.state = Some(state);
        self.window = Some(window);
        self.egui_state = Some(egui_state);
        self.egui_ctx = egui_ctx;
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

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(window), Some(egui_state)) = (&mut self.state, &self.window, &mut self.egui_state) {
                    state.update();

                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Config").show(&self.egui_ctx, |ui| {
                        ui.heading("Spring Simulator");
                        
                        ui.separator();
                        ui.heading("State");
                        ui.label(format!("y: {:.2}", state.y));
                        ui.label(format!("速度: {:.2}", state.a.abs()));
                        ui.label(format!("加速: {:.2}", state.v.abs()));

                        ui.separator();
                        ui.heading("Control");
                        if ui.button("リセット").clicked() {
                            state.reset();
                        };

                        ui.separator();
                        ui.heading("Parameters");
                        ui.add(egui::Slider::new(&mut state.g, 0.0..=10.0).text("重力"));
                        ui.add(egui::Slider::new(&mut state.k, 0.0..=10.0).text("バネの硬さ"));
                        ui.add(egui::Slider::new(&mut state.c, 0.0..=10.0).text("抵抗"));
                        
                        if ui.add(egui::Slider::new(&mut state.dt, 0.0001..=0.5).text("オイラー法 刻み")).changed() {
                            state.reset()
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
