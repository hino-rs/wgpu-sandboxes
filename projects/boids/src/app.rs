use std::sync::Arc;

use crate::boids::{Boids, INITIAL_NUM_BOIDS};
use crate::gpu::State;
use egui::Context as EguiContext;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use egui_winit::State as EguiState;
use web_time::Instant;
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    gpu: Option<State>,
    egui_ctx: EguiContext,
    egui_state: Option<EguiState>,
    last_update_time: Option<Instant>,
    boids: Option<Boids>,
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

        let gpu = pollster::block_on(State::new(Arc::clone(&window)));

        let egui_state = EguiState::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.egui_state = Some(egui_state);
        self.last_update_time = Some(Instant::now());
        self.boids = Some(Boids {
            pause: false,
            delay: 16,
            next_tick: false,
            params: crate::boids::BoidsParams::default(),
            num_boids: INITIAL_NUM_BOIDS,
        });
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
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(physical_size);
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
                    Some(gpu),
                    Some(window),
                    Some(egui_state),
                    Some(last_update_time),
                    Some(boids),
                ) = (
                    &mut self.gpu,
                    &mut self.window,
                    &mut self.egui_state,
                    &mut self.last_update_time,
                    &mut self.boids,
                ) {
                    // 最新のパラメータをGPUのUniform Bufferに書き込む
                    gpu.update_params(&boids.params);

                    let now = Instant::now();
                    let elapsed = now.duration_since(*last_update_time);
                    if !boids.pause {
                        if elapsed.as_millis() >= boids.delay as u128 {
                            gpu.update_boids(boids.num_boids);
                            *last_update_time = now;
                        }
                    } else {
                        if boids.next_tick {
                            gpu.update_boids(boids.num_boids);
                            boids.next_tick = false;
                            *last_update_time = now;
                        }
                    }

                    let raw_input = egui_state.take_egui_input(window);
                    self.egui_ctx.begin_pass(raw_input);

                    egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
                        ui.heading("Boids Control Panel");
                        ui.separator();

                        ui.checkbox(&mut boids.pause, "Pause");
                        ui.add(
                            egui::Slider::new(&mut boids.delay, 0..=100).text("Frame Delay (ms)"),
                        );
                        if boids.pause {
                            if ui.button("Step 1 Frame").clicked() {
                                boids.next_tick = true;
                            }
                        }

                        if ui.add(
                            egui::Slider::new(&mut boids.num_boids, 1..=150000).text("Num Boids"),
                        ).changed() {
                            boids.change_num_boids(gpu);
                        }

                        ui.separator();
                        ui.label("Boids Parameters");

                        ui.add(
                            egui::Slider::new(&mut boids.params.visual_range, 0.01..=0.5)
                                .text("Visual Range"),
                        );
                        ui.add(
                            egui::Slider::new(&mut boids.params.protected_range, 0.005..=0.1)
                                .text("Protected Range"),
                        );
                        ui.add(
                            egui::Slider::new(&mut boids.params.separation_weight, 0.0..=5.0)
                                .text("Separation Weight"),
                        );
                        ui.add(
                            egui::Slider::new(&mut boids.params.alignment_weight, 0.0..=3.0)
                                .text("Alignment Weight"),
                        );
                        ui.add(
                            egui::Slider::new(&mut boids.params.cohesion_weight, 0.0..=3.0)
                                .text("Cohesion Weight"),
                        );
                        ui.add(
                            egui::Slider::new(&mut boids.params.max_speed, 0.005..=0.1)
                                .text("Max Speed"),
                        );
                        ui.add(
                            egui::Slider::new(&mut boids.params.min_speed, 0.0..=0.05)
                                .text("Min Speed"),
                        );
                    });

                    let egui_output = self.egui_ctx.end_pass();
                    egui_state.handle_platform_output(window, egui_output.platform_output);

                    for (id, image_delta) in &egui_output.textures_delta.set {
                        gpu.egui_renderer
                            .update_texture(&gpu.device, &gpu.queue, *id, image_delta);
                    }

                    for id in &egui_output.textures_delta.free {
                        gpu.egui_renderer.free_texture(id);
                    }

                    let paint_jobs = self
                        .egui_ctx
                        .tessellate(egui_output.shapes, egui_output.pixels_per_point);

                    let screen_descripter = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [gpu.config.width, gpu.config.height],
                        pixels_per_point: egui_output.pixels_per_point,
                    };

                    gpu.render(&paint_jobs, &screen_descripter, boids.num_boids);
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
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

        Self {
            window: Some(window),
            gpu: Some(state),
            egui_ctx,
            egui_state: Some(egui_state),
            last_update_time: Some(Instant::now()),
            boids: Some(Boids {
                pause: false,
                delay: 16,
                next_tick: false,
                params: crate::boids::BoidsParams::default(),
                num_boids: INITIAL_NUM_BOIDS,
            }),
        }
    }
}
