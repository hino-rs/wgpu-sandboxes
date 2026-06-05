use std::sync::Arc;
use web_time::Instant;
use wgpu::SurfaceTexture;
use winit::application::ApplicationHandler;
use winit::event::{MouseButton, TouchPhase, WindowEvent};
use winit::window::Window;

use crate::common::to_ndc;
use crate::fluid::FluidSim;
use crate::gpu::GpuContext;
use crate::gui::GuiSystem;
use crate::renderer::Renderer;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MouseStateUniform {
    pub pos_x: f32,
    pub pos_y: f32,
    pub radius: f32,
    pub button: u32,
}

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    renderer: Option<Renderer>,
    gui: Option<GuiSystem>,
    fluid: Option<FluidSim>,
    last_update_time: Option<Instant>,
    mouse_state: Option<MouseState>,
    mouse_state_buffer: Option<wgpu::Buffer>,
}

#[derive(PartialEq)]
pub enum Button {
    Left,
    Right,
    None,
}

impl Button {
    pub fn to_u32(&self) -> u32 {
        match self {
            Button::Left => 1,
            Button::Right => 2,
            Button::None => 0,
        }
    }
}

pub struct MouseState {
    pub pos_x: f64,
    pub pos_y: f64,
    pub button: Button,
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
        self.mouse_state = Some(MouseState {
            pos_x: 0.0,
            pos_y: 0.0,
            button: Button::None,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let (Some(gui), Some(window)) = (&mut self.gui, &self.window)
            && gui.handle_event(window, &event)
        {
            return;
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
                if let (Some(gpu), Some(fluid), Some(last_update_time), Some(mouse_state)) =
                    (&self.gpu, &mut self.fluid, &mut self.last_update_time, &self.mouse_state)
                {
                    // 最新のパラメータをGPUのUnifrom Bufferに書き込む
                    fluid.update_params(gpu);

                    let mouse_uniform = MouseStateUniform {
                        pos_x: mouse_state.pos_x as f32,
                        pos_y: mouse_state.pos_y as f32,
                        radius: fluid.cursor_radius,
                        button: mouse_state.button.to_u32(),
                    };
                    gpu.queue.write_buffer(
                        &fluid.mouse_buffer,
                        0,
                        bytemuck::cast_slice(&[mouse_uniform]),
                    );

                    let now = Instant::now();
                    let elapsed = now.duration_since(*last_update_time);

                    let mut encoder =
                        gpu.device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Render Encoder"),
                            });

                    if !fluid.pause && elapsed.as_millis() >= fluid.delay as u128 {
                        fluid.update(&mut encoder);
                        *last_update_time = now;
                    } else {
                        if fluid.next_step {
                            fluid.update(&mut encoder);
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

            WindowEvent::Touch(touch) => {
                let (Some(mouse_state), Some(gpu)) = (&mut self.mouse_state, &self.gpu) else {
                    return;
                };

                let phase = touch.phase;

                if phase == TouchPhase::Ended {
                    mouse_state.button = Button::None;
                    return;
                }

                let position = touch.location;

                let window_size = &gpu.config;
                let width = window_size.width;
                let height = window_size.height;

                let nx = (position.x / width as f64) * 2.0 - 1.0;
                let ny = 1.0 - (position.y / height as f64) * 2.0;

                mouse_state.pos_x = nx;
                mouse_state.pos_y = ny;

                if phase == TouchPhase::Started {
                    mouse_state.button = Button::Left;
                    mouse_state.pos_x = nx;
                    mouse_state.pos_y = ny;
                }

                if phase == TouchPhase::Moved {
                    if mouse_state.button != Button::None {
                        mouse_state.pos_x = nx;
                        mouse_state.pos_y = ny;
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let (Some(mouse_state), Some(window)) = (&mut self.mouse_state, &self.window) {
                    let ndc_pos = to_ndc(&window.inner_size(), &position);
                    mouse_state.pos_x = ndc_pos[0];
                    mouse_state.pos_y = ndc_pos[1];
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(mouse_state) = &mut self.mouse_state {
                    if state.is_pressed() {
                        match button {
                            MouseButton::Left => {
                                mouse_state.button = Button::Left;
                            } 
                            MouseButton::Right => {
                                mouse_state.button = Button::Right;
                            }
                            _ => mouse_state.button = Button::None,
                        }
                    } else {
                        mouse_state.button = Button::None;
                    }
                }
            },

            _ => {}
        }
    }
}

impl App {
    fn render(&mut self) {
        let (Some(gpu), Some(fluid), Some(renderer), Some(gui), Some(window)) = (
            &self.gpu,
            &mut self.fluid,
            &mut self.renderer,
            &mut self.gui,
            &self.window,
        ) else {
            panic!("SOME APP FIELD IS NONE");
        };

        let Some(frame) = Self::get_surface_texture(gpu) else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        renderer.draw_scene(
            &mut encoder,
            &view,
            fluid.get_buffers(),
            fluid.num_particles,
        );


        gui.draw_ui(gpu, window, fluid, &mut encoder, &view);
        renderer.update_render_params(gpu, fluid);

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
            | wgpu::CurrentSurfaceTexture::Validation => None,
        }
    }
}

impl App {
    pub fn with_precreated(window: Arc<Window>, state: GpuContext) -> Self {
        let fluid = FluidSim::init(&state);
        let renderer = Renderer::init(&state, &window, &fluid.params_buffer);
        let gui = GuiSystem::init(&state, &window);

        Self {
            window: Some(window),
            gpu: Some(state),
            renderer: Some(renderer),
            gui: Some(gui),
            fluid: Some(fluid),
            last_update_time: Some(Instant::now()),
            mouse_state: Some(MouseState {
                pos_x: 0.0,
                pos_y: 0.0,
                button: Button::None,
            }),
            mouse_state_buffer: None,
        }
    }
}
