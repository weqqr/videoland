use std::sync::Arc;

use ash::extensions::khr;
use ash::vk;

use super::Error;
use crate::vk2::{Device, Instance, Surface, TextureView};

pub struct Swapchain {
    instance: Arc<Instance>,
    device: Arc<Device>,
    surface: Arc<Surface>,

    swapchain_ext: khr::Swapchain,
    swapchain: vk::SwapchainKHR,

    frames: Vec<SwapchainFrame>,

    next_frame_semaphore: vk::Semaphore,
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for frame in self.frames.drain(..) {
                self.device.raw().destroy_image_view(frame.view.raw(), None);
                self.device
                    .raw()
                    .destroy_semaphore(frame.acquire_semaphore, None);
                self.device
                    .raw()
                    .destroy_semaphore(frame.present_semaphore, None);
            }

            self.device
                .raw()
                .destroy_semaphore(self.next_frame_semaphore, None);

            if self.swapchain != vk::SwapchainKHR::null() {
                self.swapchain_ext.destroy_swapchain(self.swapchain, None);
            }
        }
    }
}

impl Swapchain {
    pub(super) unsafe fn new(
        instance: Arc<Instance>,
        device: Arc<Device>,
        surface: Arc<Surface>,
    ) -> Result<Self, Error> {
        let swapchain_ext = khr::Swapchain::new(instance.raw(), device.raw());

        let swapchain = vk::SwapchainKHR::null();

        let next_frame_semaphore = device
            .raw()
            .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)
            .unwrap();

        Ok(Self {
            instance,
            device,
            surface,

            swapchain_ext,
            swapchain,

            frames: Vec::new(),

            next_frame_semaphore,
        })
    }

    pub(super) unsafe fn resize(&mut self, size: crate::Extent2D) -> Result<(), Error> {
        let surface_format = self.surface.ext().get_physical_device_surface_formats(
            self.device.physical().raw(),
            self.surface.raw(),
        )?[0];

        let surface_capabilities = self
            .surface
            .ext()
            .get_physical_device_surface_capabilities(
                self.device.physical().raw(),
                self.surface.raw(),
            )?;

        let min_image_count = surface_capabilities
            .max_image_count
            .min(surface_capabilities.min_image_count + 1);

        let surface_resolution = if surface_capabilities.current_extent.width == u32::MAX {
            vk::Extent2D {
                width: size.width,
                height: size.height,
            }
        } else {
            surface_capabilities.current_extent
        };

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface.raw())
            .min_image_count(min_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .image_array_layers(1)
            .old_swapchain(self.swapchain);

        let old_swapchain = self.swapchain;

        self.swapchain = self
            .swapchain_ext
            .create_swapchain(&swapchain_create_info, None)?;

        self.swapchain_ext.destroy_swapchain(old_swapchain, None);

        for frame in self.frames.drain(..) {
            self.device.raw().destroy_image_view(frame.view.raw(), None);
            self.device
                .raw()
                .destroy_semaphore(frame.acquire_semaphore, None);
            self.device
                .raw()
                .destroy_semaphore(frame.present_semaphore, None);
        }

        let images = self
            .swapchain_ext
            .get_swapchain_images(self.swapchain)
            .unwrap();

        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        for (index, image) in images.into_iter().enumerate() {
            let view_create_info = vk::ImageViewCreateInfo::builder()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .subresource_range(subresource_range);
            let view = self
                .device
                .raw()
                .create_image_view(&view_create_info, None)
                .unwrap();

            let view =
                TextureView::from_managed(Arc::clone(&self.device), view, size.width, size.height);

            let acquire_semaphore = self
                .device
                .raw()
                .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)
                .unwrap();

            let present_semaphore = self
                .device
                .raw()
                .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)
                .unwrap();

            self.frames.push(SwapchainFrame {
                image,
                view,
                index: index as u32,
                acquire_semaphore,
                present_semaphore,
            });
        }

        Ok(())
    }

    pub(super) unsafe fn acquire_next_frame(&mut self) -> SwapchainFrame {
        let (index, _suboptimal) = self
            .swapchain_ext
            .acquire_next_image(
                self.swapchain,
                !0,
                self.next_frame_semaphore,
                vk::Fence::null(),
            )
            .unwrap();

        self.next_frame_semaphore = std::mem::replace(
            &mut self.frames[index as usize].acquire_semaphore,
            self.next_frame_semaphore,
        );

        self.frames[index as usize].clone()
    }

    pub(super) fn raw(&self) -> vk::SwapchainKHR {
        self.swapchain
    }

    pub(super) fn ext(&self) -> &khr::Swapchain {
        &self.swapchain_ext
    }
}

#[derive(Clone)]
pub struct SwapchainFrame {
    image: vk::Image,
    view: TextureView,
    pub(super) index: u32,
    pub(super) acquire_semaphore: vk::Semaphore,
    pub(super) present_semaphore: vk::Semaphore,
}

impl SwapchainFrame {
    pub fn raw_image(&self) -> vk::Image {
        self.image
    }

    pub fn image_view(&self) -> &TextureView {
        &self.view
    }
}
