use std::sync::Arc;
use wgpu::SurfaceTexture;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::window::Window;
use web_time::Instant;

use crate::gpu::GpuContext;
use crate::renderer::Renderer;
use crate::fluid::FluidSim;
use crate::gui::GuiSystem;

#[derive(Default)]
pub struct App {
    window:     Option<Arc<Window>>,
    gpu:        Option<GpuContext>,
    renderer:   Option<Renderer>,
    gui:        Option<GuiSystem>,
    fluid:      Option<FluidSim>,
    last_update_time: Option<Instant>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Fluid Simlator"))
                .unwrap(),
        );

        let gpu = GpuContext::init(&window);
        let fluid = FluidSim::init(&gpu);
        let renderer = Renderer::init(&gpu, &window, &fluid.params_buffer);
        let gui = GuiSystem::init(&gpu, &window);

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.renderer = Some(renderer);
        self.fluid = Some(fluid);
        self.gui = Some(gui);
        self.last_update_time = Some(Instant::now());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    )
    {
        if let (Some(gui), Some(window)) = (&mut self.gui, &self.window) {
            if gui.handle_event(window, &event) {
                return;
            }
        }

        match event {
            WindowEvent::Resized(phisical_size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(phisical_size);
                }
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let (
                    Some(window),
                    Some(gpu),
                    Some(renderer),
                    Some(gui),
                    Some(fluid),
                    Some(last_update_time),
                ) = (
                    &self.window,
                    &self.gpu,
                    &self.renderer,
                    &self.gui,
                    &mut self.fluid,
                    &mut self.last_update_time,
                ) {
                    // 最新のパラメータをGPUのUnifrom Bufferに書き込む
                    fluid.update_params(&gpu);

                    let now = Instant::now();
                    let elapsed = now.duration_since(*last_update_time);

                    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

                    if !fluid.pause {
                        if elapsed.as_millis() >= fluid.delay as u128 {
                            fluid.update(&mut encoder, &gpu);
                            *last_update_time = now;
                        }
                    } else {
                        if fluid.next_step {
                            fluid.update(&mut encoder, &gpu);
                            fluid.next_step = false;
                            *last_update_time = now;
                        }
                    }
                    gpu.queue.submit(std::iter::once(encoder.finish()));
                    self.render();

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                } else {
                    panic!("SOME APP FIELD IS NOT INITIALIZED");
                }
            }

            _ => {}
        }
    }
}

impl App {
    fn render(&mut self) {
        let (
            Some(gpu), 
            Some(fluid), 
            Some(renderer),
            Some(gui),
            Some(window)
        ) = (
            &self.gpu, 
            &mut self.fluid, 
            &mut self.renderer,
            &mut self.gui,
            &self.window,
        ) else {
            panic!("SOME APP FIELD IS NONE");
        };

        let Some(frame) = Self::get_surface_texture(&gpu) else { return; };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

        renderer.draw_scene(&mut encoder, &view, fluid.get_buffers(), fluid.num_particles);
        gui.draw_ui(&gpu, &window, fluid, &mut encoder, &view);
        renderer.update_render_params(&gpu, &fluid);

        gpu.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    fn get_surface_texture(gpu: &GpuContext) -> Option<SurfaceTexture> {
        match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                None
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                Some(frame)
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                None
            }
            _ => { None }
        }
    }
}
