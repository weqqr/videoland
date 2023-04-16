use anyhow::Result;
use ash::vk;
use gpu_allocator::vulkan::*;

pub(super) struct BufferAllocator {
    allocator: Allocator,
}

impl BufferAllocator {
    pub(super) fn new(instance: ash::Instance, device: ash::Device, physical_device: vk::PhysicalDevice) -> Result<Self> {
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance,
            device,
            physical_device,
            debug_settings: Default::default(),
            buffer_device_address: true,
        })?;

        Ok(Self { allocator })
    }
}

