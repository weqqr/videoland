use std::sync::Arc;

use ash::vk;

use crate::{Device, MemAllocator};

use super::Error;

pub struct Texture {
    device: Arc<Device>,
    allocator: MemAllocator,
    allocation: Option<gpu_alloc::MemoryBlock<vk::DeviceMemory>>,
    image: vk::Image,
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.allocator.write().unwrap().dealloc(
                gpu_alloc_ash::AshMemoryDevice::wrap(self.device.raw()),
                self.allocation.take().unwrap(),
            );

            self.device.raw().destroy_image(self.image, None);
        }
    }
}

impl Texture {
    pub(super) unsafe fn new(
        device: Arc<Device>,
        desc: &crate::TextureDesc,
    ) -> Result<Self, Error> {
        let create_info = vk::ImageCreateInfo::builder()
            .array_layers(1)
            .extent(vk::Extent3D {
                width: desc.extent.width,
                height: desc.extent.height,
                depth: desc.extent.depth,
            })
            .format(vk::Format::D24_UNORM_S8_UINT)
            .image_type(vk::ImageType::TYPE_2D)
            .mip_levels(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT);

        let image = device.raw().create_image(&create_info, None)?;
        let requirements = device.raw().get_image_memory_requirements(image);

        let allocator = device.allocator();

        let allocation = allocator.write().unwrap().alloc(
            gpu_alloc_ash::AshMemoryDevice::wrap(device.raw()),
            gpu_alloc::Request {
                size: requirements.size,
                align_mask: requirements.alignment,
                usage: gpu_alloc::UsageFlags::FAST_DEVICE_ACCESS,
                memory_types: requirements.memory_type_bits,
            },
        )?;

        device
            .raw()
            .bind_image_memory(image, *allocation.memory(), allocation.offset())?;

        Ok(Self {
            device,
            allocator,
            allocation: Some(allocation),
            image,
        })
    }

    pub(super) fn raw(&self) -> vk::Image {
        self.image
    }
}

#[derive(Clone)]
pub struct TextureView {
    is_managed: bool,
    device: Arc<Device>,
    image_view: vk::ImageView,
    width: u32,
    height: u32,
}

impl Drop for TextureView {
    fn drop(&mut self) {
        unsafe {
            if !self.is_managed {
                self.device.raw().destroy_image_view(self.image_view, None);
            }
        }
    }
}

impl TextureView {
    pub(super) unsafe fn new(
        device: Arc<Device>,
        texture: &Texture,
        desc: &crate::TextureViewDesc,
    ) -> Result<Self, Error> {
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL)
            .layer_count(vk::REMAINING_ARRAY_LAYERS)
            .base_array_layer(0)
            .level_count(vk::REMAINING_MIP_LEVELS)
            .base_mip_level(0);

        let create_info = vk::ImageViewCreateInfo::builder()
            .components(vk::ComponentMapping::default())
            .format(vk::Format::D24_UNORM_S8_UINT)
            .image(texture.image)
            .subresource_range(subresource_range.build())
            .view_type(vk::ImageViewType::TYPE_2D);

        let image_view = device.raw().create_image_view(&create_info, None)?;

        Ok(Self {
            is_managed: false,
            device,
            image_view,
            width: desc.extent.width,
            height: desc.extent.height,
        })
    }

    pub(super) unsafe fn from_managed(
        device: Arc<Device>,
        image_view: vk::ImageView,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            is_managed: true,
            device,
            image_view,
            width,
            height,
        }
    }

    pub(super) fn width(&self) -> u32 {
        self.width
    }

    pub(super) fn height(&self) -> u32 {
        self.height
    }

    pub(super) fn raw(&self) -> vk::ImageView {
        self.image_view
    }
}
