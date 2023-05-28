pub mod camera;

use crate::resources::shader::ShaderStage;
use crate::resources::{Mesh, ResourceId, Resources};
use anyhow::{Context, Result};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use smolgpu::{
    Buffer, CommandEncoder, Device, Instance, Pipeline, PipelineDesc, Surface, SurfaceConfiguration,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub use crate::renderer::camera::Camera;

const FRAMES: u32 = 2;

pub struct Renderer {
    window: Window,

    pipeline: Pipeline,
    encoder: CommandEncoder,
    surface: Surface,
    device: Device,
    instance: Instance,

    buffers: Vec<Buffer>,
}

impl Renderer {
    pub fn new(window: Window, resources: &Resources) -> Result<Self> {
        let instance =
            Instance::new(window.raw_display_handle()).context("creating vulkan instance")?;
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

        let vertex_shader = device.create_shader_module(vertex_shader.data())?;
        let fragment_shader = device.create_shader_module(fragment_shader.data())?;

        let pipeline = device.create_pipeline(&PipelineDesc {
            vertex_shader: &vertex_shader,
            fragment_shader: &fragment_shader,
        })?;

        device.destroy_shader_module(vertex_shader);
        device.destroy_shader_module(fragment_shader);

        let encoder = device.create_command_encoder(FRAMES)?;

        Ok(Self {
            window,
            instance,
            device,
            surface,
            encoder,
            pipeline,

            buffers: Vec::new(),
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
        let cmd_buffer = self.encoder.begin_immediate().unwrap();
        let buffer = self
            .device
            .upload_vertex_data_to_gpu(&cmd_buffer, bytemuck::cast_slice(mesh.data()));
        let cmd_buffer = self.encoder.finish_immediate(cmd_buffer).unwrap();
        self.device.submit_immediate(cmd_buffer);
        //
        self.buffers.push(buffer);
    }

    pub fn render<C: Camera>(&mut self, _camera: C) {
        let frame = self.surface.acquire_next_image();
        self.encoder.begin(frame);

        let size = self.window.inner_size();

        self.encoder.begin_rendering(frame.view());
        self.encoder.set_viewport(size.width, size.height);

        self.encoder.bind_pipeline(&self.pipeline);
        let data = [0; 256];

        for buffer in &self.buffers {
            self.encoder.set_push_constants(&self.pipeline, &data);
            self.encoder.bind_vertex_buffer(buffer);
            self.encoder.draw(3);
        }

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

        for buffer in self.buffers.drain(..) {
            self.device.destroy_buffer(buffer);
        }

        self.device.destroy_pipeline(&self.pipeline);
        self.device.destroy_command_encoder(&self.encoder);
    }
}
