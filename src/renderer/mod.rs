use crate::gapi::pipeline::PipelineDesc;
use crate::gapi::*;
use crate::resources::shader::ShaderStage;
use crate::resources::{ResourceId, Resources, Mesh};
use anyhow::{Context, Result};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use tracing::instrument;
use winit::dpi::PhysicalSize;
use winit::window::Window;

// FIXME: this value should be determined automatically by gapi
const FRAMES: u32 = 2;

pub struct Renderer {
    window: Window,

    encoder: CommandEncoder,
    surface: Surface,
    device: Device,
    instance: Instance,
}

impl Renderer {
    pub fn new(window: Window, resources: &Resources) -> Result<Self> {
        let instance = Instance::new(window.raw_display_handle()).context("create instance")?;
        let device = instance.create_device()?;
        let mut surface = instance.create_surface(
            &device,
            window.raw_display_handle(),
            window.raw_window_handle(),
        )?;

        let size = window.inner_size();
        surface.configure(SurfaceConfiguration {
            frames: FRAMES,
            width: size.width,
            height: size.height,
        });

        let shader_id = ResourceId::new("/dsots/shaders/test.hlsl");

        let vertex_shader = resources.load_shader(shader_id.clone(), ShaderStage::Vertex)?;
        let fragment_shader = resources.load_shader(shader_id, ShaderStage::Fragment)?;

        let vertex_shader = device.create_shader_module(&vertex_shader)?;
        let fragment_shader = device.create_shader_module(&fragment_shader)?;

        let pipeline = device.create_pipeline(&PipelineDesc {
            vertex_shader,
            fragment_shader,
        })?;

        let encoder = device.create_command_encoder(FRAMES)?;

        Ok(Self {
            window,
            instance,
            device,
            surface,
            encoder,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface.configure(SurfaceConfiguration {
            frames: FRAMES,
            width: size.width,
            height: size.height,
        })
    }

    pub fn add_mesh(&mut self, mesh: &Mesh) {
        let buf = self.device.upload_vertex_data_to_gpu(bytemuck::cast_slice(mesh.data()));
    }

    #[instrument(skip(self))]
    pub fn render(&mut self) {
        let frame = self.surface.acquire_next_image();
        self.encoder.begin(frame);

        self.encoder.begin_rendering(frame.view());

        self.encoder.end_rendering();

        self.device.finish_frame(&self.encoder, frame);
        frame.present();
        self.device.wait_for_sync();
    }
}

impl Renderer {
    pub fn window(&self) -> &Window {
        &self.window
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_for_sync();
        self.device.destroy_command_encoder(&mut self.encoder);
    }
}
