use std::sync::Arc;

use ash::vk;

use crate::{Device, Error, MemAllocator};

pub struct Buffer {
    device: Arc<Device>,
    allocator: MemAllocator,
    allocation: Option<gpu_alloc::MemoryBlock<vk::DeviceMemory>>,
    buffer: vk::Buffer,
    len: u64,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.allocator.write().unwrap().dealloc(
                gpu_alloc_ash::AshMemoryDevice::wrap(self.device.raw()),
                self.allocation.take().unwrap(),
            );

            self.device.raw().destroy_buffer(self.buffer, None);
        }
    }
}

impl Buffer {
    pub(super) unsafe fn new(
        device: Arc<Device>,
        allocation: crate::BufferAllocation,
    ) -> Result<Self, Error> {
        let len = allocation.size;

        let create_info = vk::BufferCreateInfo::builder()
            .size(allocation.size)
            .usage(buffer_usage_to_vk(allocation.usage));

        let buffer = device.raw().create_buffer(&create_info, None)?;
        let requirements = device.raw().get_buffer_memory_requirements(buffer);

        let allocator = device.allocator();

        let allocation = allocator.write().unwrap().alloc(
            gpu_alloc_ash::AshMemoryDevice::wrap(device.raw()),
            gpu_alloc::Request {
                size: requirements.size,
                align_mask: requirements.alignment,
                usage: match allocation.location {
                    crate::BufferLocation::Cpu => gpu_alloc::UsageFlags::UPLOAD,
                    crate::BufferLocation::Gpu => gpu_alloc::UsageFlags::FAST_DEVICE_ACCESS,
                },
                memory_types: requirements.memory_type_bits,
            },
        )?;

        device
            .raw()
            .bind_buffer_memory(buffer, *allocation.memory(), allocation.offset())?;

        Ok(Self {
            device,
            allocator,
            buffer,
            allocation: Some(allocation),
            len,
        })
    }

    pub fn write_data(&mut self, offset: u64, data: &[u8]) {
        unsafe {
            self.allocation
                .as_mut()
                .unwrap()
                .write_bytes(
                    gpu_alloc_ash::AshMemoryDevice::wrap(self.device.raw()),
                    offset,
                    data,
                )
                .unwrap();
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u64 {
        self.len
    }

    pub(super) fn raw(&self) -> vk::Buffer {
        self.buffer
    }
}

fn buffer_usage_to_vk(usage: crate::BufferUsage) -> vk::BufferUsageFlags {
    let mut vk_usage = vk::BufferUsageFlags::empty();

    if usage.contains(crate::BufferUsage::VERTEX) {
        vk_usage |= vk::BufferUsageFlags::VERTEX_BUFFER;
    }

    if usage.contains(crate::BufferUsage::INDEX) {
        vk_usage |= vk::BufferUsageFlags::INDEX_BUFFER;
    }

    if usage.contains(crate::BufferUsage::TRANSFER_SRC) {
        vk_usage |= vk::BufferUsageFlags::TRANSFER_SRC;
    }

    if usage.contains(crate::BufferUsage::TRANSFER_DST) {
        vk_usage |= vk::BufferUsageFlags::TRANSFER_DST;
    }

    vk_usage
}
