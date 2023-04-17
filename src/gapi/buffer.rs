use anyhow::Result;
use ash::vk;
use gpu_allocator::{vulkan::*, MemoryLocation};

pub(super) struct BufferAllocator {
    device: ash::Device,
    allocator: Allocator,
}

impl BufferAllocator {
    pub(super) fn new(instance: ash::Instance, device: ash::Device, physical_device: vk::PhysicalDevice) -> Result<Self> {
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance,
            device: device.clone(),
            physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
        })?;

        Ok(Self { device, allocator })
    }

    pub(super) fn allocate_buffer(&mut self, size: usize) -> Buffer {
        let create_info = vk::BufferCreateInfo::builder()
            .size(size as u64)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER);

        let buffer = unsafe { self.device.create_buffer(&create_info, None) }.unwrap();
        let requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let allocation = self.allocator
            .allocate(&AllocationCreateDesc {
                name: "buffer",
                requirements,
                location: MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .unwrap();

        unsafe {
            self.device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset()).unwrap();
        }

        Buffer {
            buffer,
            allocation,
        }
    }

    pub(super) fn free_buffer(&mut self, buffer: Buffer) {
        self.allocator.free(buffer.allocation).unwrap();
        unsafe {
            self.device.destroy_buffer(buffer.buffer, None);
        }
    }
}

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Allocation,
}

impl Buffer {
    pub fn copy_from_slice(&mut self, slice: &[u8]) {
        self.allocation.mapped_slice_mut().unwrap().copy_from_slice(slice)
    }
}
