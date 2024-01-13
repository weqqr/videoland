#![allow(dead_code)]

#[cfg(feature = "vk")]
mod vk;

#[cfg(feature = "vk")]
use crate::vk as gapi;

use bitflags::bitflags;
use raw_window_handle::HasWindowHandle;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct BufferUsage(u32);

bitflags! {
    impl BufferUsage: u32 {
        const VERTEX = 1 << 0;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BufferLocation {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy)]
pub struct BufferAllocation {
    pub usage: BufferUsage,
    pub location: BufferLocation,
    pub size: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("GAPI error: {0}")]
    Api(#[from] gapi::Error),
}

pub struct Device {
    device: Arc<gapi::Device2>,
}

impl Device {
    pub fn new<W>(window: W) -> Result<Self, Error>
    where
        W: HasWindowHandle,
    {
        let device = gapi::Device2::new(window).unwrap();

        Ok(Self {
            device: Arc::new(device),
        })
    }

    pub fn resize_swapchain(&self, size: impl Into<Extent2D>) -> Result<(), Error> {
        Ok(self.device.resize_swapchain(size.into())?)
    }

    pub fn acquire_next_image(&self) -> SwapchainFrame {
        SwapchainFrame::new(self.device.acquire_next_frame())
    }

    pub fn create_shader_module(&self, data: &[u32]) -> Result<ShaderModule, Error> {
        let shader_module = unsafe { self.device.device.create_shader_module(data)? };

        Ok(ShaderModule {
            device: Arc::clone(&self.device),
            shader_module,
        })
    }

    pub fn create_pipeline(&self, desc: &PipelineDesc) -> Result<Pipeline, Error> {
        let pipeline = unsafe { self.device.device.create_pipeline(desc)? };

        Ok(Pipeline {
            device: Arc::clone(&self.device),
            pipeline,
        })
    }

    pub fn create_buffer(&self, allocation: BufferAllocation) -> Result<Buffer, Error> {
        let buffer = unsafe { self.device.device.create_buffer(allocation)? };

        Ok(Buffer {
            device: Arc::clone(&self.device),
            buffer: RwLock::new(buffer),
        })
    }

    pub fn create_texture(&self, desc: &TextureDesc) -> Result<Texture, Error> {
        let cbuf = self
            .device
            .command_encoder
            .read()
            .unwrap()
            .current_command_buffer();
        let texture = unsafe { self.device.device.create_texture(cbuf, desc)? };

        Ok(Texture { texture })
    }

    pub fn destroy_texture(&self, texture: &mut Texture) {
        unsafe {
            self.device.device.destroy_texture(&mut texture.texture);
        }
    }

    pub fn create_texture_view(
        &self,
        texture: &Texture,
        desc: &TextureViewDesc,
    ) -> Result<TextureView, Error> {
        let texture_view = unsafe {
            self.device
                .device
                .create_texture_view(&texture.texture, desc)?
        };

        Ok(TextureView { texture_view })
    }

    pub fn destroy_texture_view(&self, texture_view: &mut TextureView) {
        unsafe {
            self.device
                .device
                .destroy_texture_view(&mut texture_view.texture_view);
        }
    }

    pub fn begin_command_buffer(&mut self, frame: &SwapchainFrame) -> CommandBuffer {
        CommandBuffer::new(self.device.begin_command_buffer(&frame.frame))
    }

    pub fn submit_frame(
        &self,
        command_buffer: CommandBuffer,
        frame: &SwapchainFrame,
    ) -> Result<(), Error> {
        unsafe {
            self.device.device.submit_frame(
                &self.device.command_encoder.read().unwrap(),
                command_buffer.command_buffer,
                &self.device.swapchain.read().unwrap(),
                &frame.frame,
            )?;
        }

        Ok(())
    }
}

pub struct SwapchainFrame {
    frame: gapi::SwapchainFrame,
}

impl SwapchainFrame {
    fn new(frame: gapi::SwapchainFrame) -> Self {
        Self { frame }
    }

    pub fn image_view(&self) -> TextureView {
        TextureView::new(self.frame.image_view())
    }
}

pub struct RenderPassDesc<'a> {
    pub color_attachment: &'a TextureView,
    pub depth_attachment: &'a TextureView,
    pub render_area: Extent2D,
}

pub struct CommandBuffer {
    pub(crate) command_buffer: gapi::CommandBuffer,
}

impl CommandBuffer {
    fn new(command_buffer: gapi::CommandBuffer) -> Self {
        Self { command_buffer }
    }

    pub fn begin(&self) {
        unsafe {
            self.command_buffer.begin();
        }
    }

    pub fn texture_barrier(&self, texture: &Texture, old: TextureLayout, new: TextureLayout) {
        unsafe {
            self.command_buffer
                .texture_barrier(&texture.texture, old, new)
        }
    }

    pub fn begin_rendering(&self, desc: RenderPassDesc) {
        unsafe {
            self.command_buffer.begin_rendering(desc);
        }
    }

    pub fn end_rendering(&self) {
        unsafe {
            self.command_buffer.end_rendering();
        }
    }

    pub fn set_viewport(&self, extent: impl Into<Extent2D>) {
        unsafe {
            self.command_buffer.set_viewport(extent.into());
        }
    }

    pub fn bind_pipeline(&self, pipeline: &Pipeline) {
        unsafe {
            self.command_buffer.bind_pipeline(&pipeline.pipeline);
        }
    }

    pub fn bind_vertex_buffer(&self, buffer: &Buffer) {
        let buffer = buffer.buffer.read().unwrap();

        unsafe {
            self.command_buffer.bind_vertex_buffer(&buffer);
        }
    }

    pub fn set_push_constants(&self, pipeline: &Pipeline, offset: u32, constants: &[u8]) {
        unsafe {
            self.command_buffer
                .set_push_constants(&pipeline.pipeline, offset, constants);
        }
    }

    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.command_buffer
                .draw(vertex_count, instance_count, first_vertex, first_instance);
        }
    }
}

pub struct ShaderModule {
    device: Arc<gapi::Device2>,
    shader_module: gapi::ShaderModule,
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_shader_module(&self.shader_module);
        }
    }
}

pub struct PipelineDesc<'a> {
    pub vertex: &'a ShaderModule,
    pub fragment: &'a ShaderModule,
    pub vertex_layout: VertexBufferLayout<'a>,
}

pub struct Pipeline {
    device: Arc<gapi::Device2>,
    pipeline: gapi::Pipeline,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_pipeline(&self.pipeline);
        }
    }
}

pub struct Buffer {
    device: Arc<gapi::Device2>,
    buffer: RwLock<gapi::Buffer>,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_buffer(&mut self.buffer.write().unwrap());
        }
    }
}

impl Buffer {
    pub fn write_data(&self, offset: u64, data: &[u8]) {
        unsafe { self.buffer.write().unwrap().write_data(offset, data) }
    }

    pub fn len(&self) -> u64 {
        self.buffer.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct TextureDesc {
    pub extent: Extent3D,
}

pub struct Texture {
    texture: gapi::Texture,
}

pub struct TextureViewDesc {
    pub extent: Extent3D,
}

pub struct TextureView {
    texture_view: gapi::TextureView,
}

impl TextureView {
    fn new(image_view: gapi::TextureView) -> Self {
        Self {
            texture_view: image_view,
        }
    }
}

#[derive(Clone, Copy)]
pub enum TextureLayout {
    Undefined,
    General,
    Color,
    DepthStencil,
    TransferSrc,
    TransferDst,
}

#[derive(Clone, Copy)]
pub enum VertexFormat {
    Float32x1,
    Float32x2,
    Float32x3,
    Float32x4,
}

#[derive(Clone)]
pub struct VertexAttribute {
    pub binding: u32,
    pub location: u32,
    pub offset: u32,
    pub format: VertexFormat,
}

#[derive(Clone)]
pub struct VertexBufferLayout<'a> {
    pub attributes: &'a [VertexAttribute],
    pub stride: u32,
}
