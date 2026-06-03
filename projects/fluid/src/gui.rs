use egui::Context as EguiContext;
use egui_wgpu::Renderer as EguiRenderer;
use egui_wgpu::RendererOptions;
use egui_winit::State as EguiState;
use winit::event::WindowEvent;
use winit::window::Window;

use crate::fluid::FluidSim;
use crate::gpu::GpuContext;

pub struct GuiSystem {
    pub egui_ctx: EguiContext,
    pub egui_state: EguiState,
    pub egui_renderer: EguiRenderer,
}

impl GuiSystem {
    pub fn init(gpu: &GpuContext, window: &Window) -> Self {
        let egui_ctx = EguiContext::default();
        let egui_state = EguiState::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            None,
            None,
            None,
        );
        let egui_renderer =
            EguiRenderer::new(&gpu.device, gpu.config.format, RendererOptions::default());

        Self {
            egui_ctx,
            egui_state,
            egui_renderer,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.egui_state.on_window_event(window, event);
        response.consumed
    }

    pub fn draw_ui(
        &mut self,
        gpu: &GpuContext,
        window: &Window,
        fluid: &mut FluidSim,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let raw_input = self.egui_state.take_egui_input(window);
        self.egui_ctx.begin_pass(raw_input);

        egui::Window::new("Configs").show(&self.egui_ctx, |ui| {
            ui.heading("Fluid Simulator Control Panel");
            ui.label(format!("Num Particles: {}", fluid.num_particles));
            ui.separator();

            ui.checkbox(&mut fluid.pause, "Pause");
            ui.add(egui::Slider::new(&mut fluid.delay, 0..=100).text("Frame Delay (ms)"));
            if fluid.pause && ui.button("Step 1 Frame").clicked() {
                fluid.next_step = true;
            }

            ui.add(egui::Slider::new(&mut fluid.glow_width, 0.1..=100.0).text("Glow Width"));

            ui.add(
                egui::Slider::new(&mut fluid.cursor_radius, 0.0..=0.5)
                    .text("Cursor Radius"),
            );

            ui.separator();
            ui.add(
                egui::Slider::new(&mut fluid.params.visual_range, 0.01..=0.30)
                    .text("Kernel Radius (h)"),
            );
            ui.add(
                egui::Slider::new(&mut fluid.params.protected_range, 0.0..=2.0)
                    .text("Look-Ahead Prediction"),
            );
            ui.add(
                egui::Slider::new(&mut fluid.params.separation_weight, 0.0..=10.0)
                    .text("Pressure Strength (k)"),
            );
            ui.add(
                egui::Slider::new(&mut fluid.params.alignment_weight, 0.0..=10.0)
                    .text("Viscosity (Friction)"),
            );
            ui.add(
                egui::Slider::new(&mut fluid.params.cohesion_weight, 0.0..=10.0)
                    .text("Rest Density Scale"),
            );
            ui.add(
                egui::Slider::new(&mut fluid.params.max_speed, 0.00..=0.05)
                    .text("Max Speed Limit"),
            );
            ui.add(
                egui::Slider::new(&mut fluid.params.min_speed, 0.00..=0.02).text("Min Speed Limit"),
            );
        });

        let egui_output = self.egui_ctx.end_pass();
        self.egui_state
            .handle_platform_output(window, egui_output.platform_output);

        for (id, image_delta) in &egui_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&gpu.device, &gpu.queue, *id, image_delta);
        }
        for id in &egui_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let paint_jobs = self
            .egui_ctx
            .tessellate(egui_output.shapes, egui_output.pixels_per_point);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gpu.config.width, gpu.config.height],
            pixels_per_point: egui_output.pixels_per_point,
        };

        self.egui_renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut egui_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Egui Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();

            self.egui_renderer
                .render(&mut egui_pass, &paint_jobs, &screen_descriptor);
        }
    }
}
