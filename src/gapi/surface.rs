use anyhow::Result;
use ash::extensions::khr;
use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::gapi::ImageView;

pub struct SwapchainFrame {
    khr_swapchain: khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    queue: vk::Queue,

    image_index: u32,
    pub(super) image: vk::Image,
    view: vk::ImageView,
    format: vk::Format,
    pub(super) acquire_semaphore: vk::Semaphore,
    pub(super) present_semaphore: vk::Semaphore,
    size: [u32; 2],
}

impl SwapchainFrame {
    pub fn view(&self) -> ImageView {
        ImageView {
            image_view: self.view,
            width: self.size[0],
            height: self.size[1],
        }
    }

    pub fn present(&self) {
        let wait_semaphores = &[self.present_semaphore];
        let swapchains = &[self.swapchain];
        let image_indices = &[self.image_index];

        let present_info = vk::PresentInfoKHR::builder()
            .swapchains(swapchains)
            .wait_semaphores(wait_semaphores)
            .image_indices(image_indices);

        unsafe {
            self.khr_swapchain
                .queue_present(self.queue, &present_info)
                .unwrap();
        }
    }
}

pub struct Surface {
    device: ash::Device,
    queue: vk::Queue,

    surface: vk::SurfaceKHR,
    khr_surface: khr::Surface,
    swapchain: vk::SwapchainKHR,
    khr_swapchain: khr::Swapchain,

    next_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
    frames: Vec<SwapchainFrame>,
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.present_semaphore, None);
            self.device.destroy_semaphore(self.next_semaphore, None);

            for frame in self.frames.drain(..) {
                self.device.destroy_image_view(frame.view, None);
                self.device.destroy_semaphore(frame.acquire_semaphore, None);
            }

            if self.swapchain != vk::SwapchainKHR::null() {
                self.khr_swapchain.destroy_swapchain(self.swapchain, None);
            }
            self.khr_surface.destroy_surface(self.surface, None);
        }
    }
}

pub struct SurfaceConfiguration {
    pub frames: u32,
    pub width: u32,
    pub height: u32,
}

impl Surface {
    pub(super) fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &ash::Device,
        queue: vk::Queue,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self> {
        let khr_surface = khr::Surface::new(entry, instance);

        let surface = unsafe {
            ash_window::create_surface(entry, instance, display_handle, window_handle, None)?
        };

        let khr_swapchain = khr::Swapchain::new(instance, device);

        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
        let next_semaphore = unsafe {
            device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap()
        };

        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
        let present_semaphore = unsafe {
            device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap()
        };

        Ok(Surface {
            device: device.clone(),
            queue,

            surface,
            khr_surface,
            swapchain: vk::SwapchainKHR::null(),
            khr_swapchain,

            next_semaphore,
            present_semaphore,
            frames: Vec::new(),
        })
    }

    pub fn configure(&mut self, configuration: SurfaceConfiguration) {
        let queue_families = [0];

        let format = vk::Format::B8G8R8A8_SRGB;

        let usage = vk::ImageUsageFlags::empty() | vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface)
            .min_image_count(configuration.frames)
            .image_format(format)
            .image_extent(vk::Extent2D {
                width: configuration.width,
                height: configuration.height,
            })
            .image_array_layers(1)
            .image_usage(usage)
            .queue_family_indices(&queue_families)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .old_swapchain(self.swapchain);

        self.swapchain = unsafe {
            self.khr_swapchain
                .create_swapchain(&create_info, None)
                .unwrap()
        };

        for frame in self.frames.drain(..) {
            unsafe {
                self.device.destroy_image_view(frame.view, None);
                self.device.destroy_semaphore(frame.acquire_semaphore, None);
            }
        }

        let images = unsafe {
            self.khr_swapchain
                .get_swapchain_images(self.swapchain)
                .unwrap()
        };
        let size = [configuration.width, configuration.height];
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
                .format(format)
                .subresource_range(subresource_range);
            let view = unsafe {
                self.device
                    .create_image_view(&view_create_info, None)
                    .unwrap()
            };
            let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
            let acquire_semaphore = unsafe {
                self.device
                    .create_semaphore(&semaphore_create_info, None)
                    .unwrap()
            };

            self.frames.push(SwapchainFrame {
                khr_swapchain: self.khr_swapchain.clone(),
                swapchain: self.swapchain,
                queue: self.queue,

                image_index: index as u32,
                image,
                view,
                format,
                present_semaphore: self.present_semaphore,
                acquire_semaphore,
                size,
            });
        }
    }

    pub fn acquire_next_image(&mut self) -> &SwapchainFrame {
        let (index, _suboptimal) = unsafe {
            self.khr_swapchain
                .acquire_next_image(self.swapchain, !0, self.next_semaphore, vk::Fence::null())
                .unwrap()
        };
        self.next_semaphore = std::mem::replace(
            &mut self.frames[index as usize].acquire_semaphore,
            self.next_semaphore,
        );
        &self.frames[index as usize]
    }
}
