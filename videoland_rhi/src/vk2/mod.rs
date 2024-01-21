mod buffer;
mod command;
mod device;
mod instance;
mod pipeline;
mod surface;
mod swapchain;
mod texture;

use std::sync::{Arc, RwLock};

pub use buffer::*;
pub use command::*;
pub use device::*;
pub use instance::*;
pub use pipeline::*;
pub use surface::*;
pub use swapchain::*;
pub use texture::*;

use raw_window_handle::HasWindowHandle;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Vulkan instance has no supported devices")]
    NoDevices,

    #[error("Vulkan error: {0}")]
    Vulkan(#[from] ash::vk::Result),

    #[error("Vulkan loading error: {0}")]
    Loading(#[from] ash::LoadingError),

    #[error("memory allocation error: {0}")]
    Allocation(#[from] gpu_alloc::AllocationError),
}

#[derive(Clone)]
pub struct Context {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    device: Arc<Device>,
    swapchain: Arc<RwLock<Swapchain>>,
    command_encoder: Arc<RwLock<CommandEncoder>>,
}

impl Context {
    pub fn new<W>(window: W) -> Result<Self, Error>
    where
        W: HasWindowHandle,
    {
        unsafe {
            let instance = Arc::new(Instance::new()?);
            let surface = Arc::new(Surface::new(Arc::clone(&instance), window)?);
            let physical_device = instance.get_physical_device(&surface)?;
            let device = Arc::new(Device::new(Arc::clone(&instance), physical_device)?);
            let swapchain = Arc::new(RwLock::new(Swapchain::new(
                Arc::clone(&instance),
                Arc::clone(&device),
                Arc::clone(&surface),
            )?));
            let command_encoder =
                Arc::new(RwLock::new(CommandEncoder::new(Arc::clone(&device), 0, 2)?));

            Ok(Self {
                instance,
                surface,
                device,
                swapchain,
                command_encoder,
            })
        }
    }

    pub fn create_buffer(&self, allocation: crate::BufferAllocation) -> Buffer {
        unsafe { Buffer::new(Arc::clone(&self.device), allocation).unwrap() }
    }

    pub fn create_shader_module(&self, spirv: &[u32]) -> ShaderModule {
        unsafe { ShaderModule::new(Arc::clone(&self.device), spirv).unwrap() }
    }

    pub fn create_pipeline(&self, desc: &crate::PipelineDesc) -> Pipeline {
        unsafe { Pipeline::new(Arc::clone(&self.device), desc).unwrap() }
    }

    pub fn create_texture(&self, desc: &crate::TextureDesc) -> Texture {
        unsafe { Texture::new(Arc::clone(&self.device), desc).unwrap() }
    }

    pub fn create_texture_view(
        &self,
        texture: &Texture,
        desc: &crate::TextureViewDesc,
    ) -> TextureView {
        unsafe { TextureView::new(Arc::clone(&self.device), texture, desc).unwrap() }
    }

    pub fn resize_swapchain(&self, extent: crate::Extent2D) {
        unsafe { self.swapchain.write().unwrap().resize(extent).unwrap() }
    }

    pub fn acquire_next_frame(&self) -> SwapchainFrame {
        unsafe { self.swapchain.write().unwrap().acquire_next_frame() }
    }

    pub fn submit_frame(&self, command_buffer: CommandBuffer, frame: &SwapchainFrame) {
        unsafe {
            self.device
                .submit_frame(command_buffer, &self.swapchain.read().unwrap(), frame)
                .unwrap()
        }
    }

    pub fn begin_command_buffer(&self, frame: &SwapchainFrame) -> CommandBuffer {
        unsafe { self.command_encoder.write().unwrap().begin(frame) }
    }
}
