use anyhow::Result;
use ash::vk;
use gpu_alloc::{Config, GpuAllocator, MemoryBlock, Request, UsageFlags};
use gpu_alloc_ash::AshMemoryDevice;

#[derive(Debug, Copy, Clone)]
pub enum BufferLocation {
    Gpu,
    Cpu,
}

impl BufferLocation {
    fn to_usage(self) -> UsageFlags {
        match self {
            BufferLocation::Gpu => UsageFlags::FAST_DEVICE_ACCESS,
            BufferLocation::Cpu => UsageFlags::UPLOAD,
        }
    }
}

pub(super) struct BufferAllocator {
    device: ash::Device,
    allocator: GpuAllocator<vk::DeviceMemory>,
}

impl BufferAllocator {
    pub(super) fn new(
        instance: ash::Instance,
        device: ash::Device,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        let mut properties = unsafe {
            gpu_alloc_ash::device_properties(&instance, vk::API_VERSION_1_3, physical_device)?
        };

        properties.buffer_device_address = false;

        let allocator = GpuAllocator::new(Config::i_am_prototyping(), properties);

        Ok(Self { device, allocator })
    }

    pub(super) fn allocate_buffer(&mut self, size: usize, location: BufferLocation, usage: vk::BufferUsageFlags) -> Buffer {
        let create_info = vk::BufferCreateInfo::builder()
            .size(size as u64)
            .usage(usage);

        let buffer = unsafe { self.device.create_buffer(&create_info, None) }.unwrap();
        let requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let allocation = unsafe {
            self.allocator
                .alloc(
                    AshMemoryDevice::wrap(&self.device),
                    Request {
                        size: requirements.size,
                        align_mask: requirements.alignment,
                        usage: location.to_usage(),
                        memory_types: requirements.memory_type_bits,
                    },
                )
                .unwrap()
        };

        unsafe {
            self.device
                .bind_buffer_memory(buffer, *allocation.memory(), allocation.offset())
                .unwrap();
        }

        Buffer {
            device: self.device.clone(),
            buffer,
            allocation,
        }
    }

    pub(super) fn free_buffer(&mut self, buffer: Buffer) {
        unsafe {
            self.allocator
                .dealloc(AshMemoryDevice::wrap(&self.device), buffer.allocation);
            self.device.destroy_buffer(buffer.buffer, None);
        }
    }
}

pub struct Buffer {
    device: ash::Device,
    pub(super) buffer: vk::Buffer,
    allocation: MemoryBlock<vk::DeviceMemory>,
}

impl Buffer {
    pub fn copy_from_slice(&mut self, slice: &[u8]) {
        unsafe {
            self.allocation
                .write_bytes(AshMemoryDevice::wrap(&self.device), 0, slice)
                .unwrap();
        }
    }
}
