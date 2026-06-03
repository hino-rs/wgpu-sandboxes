use egui_wgpu::Renderer as EguiRenderer;
use egui_wgpu::RendererOptions;

use crate::gpu::GpuContext;

pub struct GuiSystem {
    pub egui_renderer: EguiRenderer,
}

impl GuiSystem {
    pub fn init(gpu: &GpuContext) -> Self {
        let egui_renderer = EguiRenderer::new(&gpu.device, gpu.config.format, RendererOptions::default());

        Self {
            egui_renderer,
        }
    }
}