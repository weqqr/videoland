use std::ffi::CStr;

use anyhow::{anyhow, Context, Result};
use ash::extensions::{ext, khr};
use ash::vk::{self, DebugUtilsMessageSeverityFlagsEXT};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

struct PhysicalDevice {
    name: String,
    device: vk::PhysicalDevice,
    properties: vk::PhysicalDeviceProperties,
}

pub struct Instance {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils: ext::DebugUtils,

    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}

impl Instance {
    pub fn new(display_handle: RawDisplayHandle) -> Result<Self> {
        let instance = unsafe {
            let entry = ash::Entry::load()?;

            let khronos_validation =
                CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap();
            let layers = vec![khronos_validation.as_ptr()];

            let mut extensions = vec![ext::DebugUtils::name().as_ptr()];
            extensions
                .extend_from_slice(ash_window::enumerate_required_extensions(display_handle)?);

            let application_info = vk::ApplicationInfo::builder().api_version(vk::API_VERSION_1_3);
            let create_info = vk::InstanceCreateInfo::builder()
                .enabled_extension_names(&extensions)
                .enabled_layer_names(&layers)
                .application_info(&application_info);
            let instance = entry.create_instance(&create_info, None)?;

            let debug_utils = ext::DebugUtils::new(&entry, &instance);

            let severity = DebugUtilsMessageSeverityFlagsEXT::empty()
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;

            let ty = vk::DebugUtilsMessageTypeFlagsEXT::empty()
                | vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE;

            let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(severity)
                .message_type(ty)
                .pfn_user_callback(Some(vulkan_debug_callback));

            let debug_utils_messenger =
                debug_utils.create_debug_utils_messenger(&create_info, None)?;

            Self {
                entry,
                instance,
                debug_utils,
                debug_utils_messenger,
            }
        };

        Ok(instance)
    }

    fn evaluate_device(&self, phd: vk::PhysicalDevice) -> Option<PhysicalDevice> {
        let mut properties2 = vk::PhysicalDeviceProperties2::builder();
        unsafe {
            self.instance
                .get_physical_device_properties2(phd, &mut properties2);
        }

        let properties = properties2.properties;

        let data = properties
            .device_name
            .iter()
            .map_while(|&c| (c != 0).then_some(c as u8))
            .collect();
        let name = String::from_utf8(data).unwrap();

        // TODO: analyze device features

        tracing::info!(device = name);

        Some(PhysicalDevice {
            name,
            device: phd,
            properties: properties2.properties,
        })
    }

    pub fn create_device(&self) -> Result<Device> {
        let phd = unsafe {
            self.instance
                .enumerate_physical_devices()
                .unwrap()
                .into_iter()
                .find_map(|phd| self.evaluate_device(phd))
                .context(anyhow!("no suitable device found"))?
        };

        // FIXME:
        let graphics_queue_family_index = 0;

        let create_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_family_index)
            .queue_priorities(&[1.0])
            .build();

        let queue_create_infos = &[create_info];

        let device_extensions = vec![
            vk::KhrDynamicRenderingFn::name().as_ptr(),
            vk::KhrSwapchainFn::name().as_ptr(),
            vk::KhrTimelineSemaphoreFn::name().as_ptr(),
        ];

        let mut khr_dynamic_rendering =
            vk::PhysicalDeviceDynamicRenderingFeaturesKHR::builder().dynamic_rendering(true);
        let mut khr_timeline_semaphore =
            vk::PhysicalDeviceTimelineSemaphoreFeaturesKHR::builder().timeline_semaphore(true);
        let create_info = vk::DeviceCreateInfo::builder()
            .enabled_extension_names(&device_extensions)
            .queue_create_infos(queue_create_infos)
            .push_next(&mut khr_dynamic_rendering)
            .push_next(&mut khr_timeline_semaphore);

        let device = unsafe {
            self.instance
                .create_device(phd.device, &create_info, None)?
        };

        let khr_dynamic_rendering = khr::DynamicRendering::new(&self.instance, &device);
        let khr_timeline_semaphore = khr::TimelineSemaphore::new(&self.instance, &device);

        let queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        let mut semaphore_type_create_info = vk::SemaphoreTypeCreateInfo::builder()
            .semaphore_type(vk::SemaphoreType::TIMELINE_KHR)
            .initial_value(0);
        let create_info =
            vk::SemaphoreCreateInfo::builder().push_next(&mut semaphore_type_create_info);
        let timeline_semaphore = unsafe { device.create_semaphore(&create_info, None)? };

        Ok(Device {
            device,
            khr_dynamic_rendering,
            khr_timeline_semaphore,
            graphics_queue_family_index,

            timeline_semaphore,
            queue,

            sync: 0,
        })
    }

    pub fn create_surface(
        &self,
        device: &Device,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Surface> {
        let khr_surface = khr::Surface::new(&self.entry, &self.instance);

        let surface = unsafe {
            ash_window::create_surface(
                &self.entry,
                &self.instance,
                display_handle,
                window_handle,
                None,
            )?
        };

        let khr_swapchain = khr::Swapchain::new(&self.instance, &device.device);

        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
        let next_semaphore = unsafe {
            device
                .device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap()
        };

        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
        let present_semaphore = unsafe {
            device
                .device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap()
        };

        Ok(Surface {
            device: device.device.clone(),
            queue: device.queue,

            surface,
            khr_surface,
            swapchain: vk::SwapchainKHR::null(),
            khr_swapchain,

            next_semaphore,
            present_semaphore,
            frames: Vec::new(),
        })
    }
}

pub struct Device {
    device: ash::Device,
    khr_dynamic_rendering: khr::DynamicRendering,
    khr_timeline_semaphore: khr::TimelineSemaphore,
    graphics_queue_family_index: u32,

    timeline_semaphore: vk::Semaphore,
    // present_semaphore: vk::Semaphore,
    queue: vk::Queue,

    sync: u64,
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.wait_for_sync();
            self.device.destroy_semaphore(self.timeline_semaphore, None);
            self.device.destroy_device(None);
        }
    }
}

impl Device {
    pub fn create_command_encoder(&self, frames: u32) -> Result<CommandEncoder> {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.graphics_queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let cmd_pool = unsafe { self.device.create_command_pool(&create_info, None)? };

        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(frames)
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let cmd_bufs = unsafe { self.device.allocate_command_buffers(&allocate_info)? };

        Ok(CommandEncoder {
            device: self.device.clone(),
            cmd_pool,
            cmd_bufs,
        })
    }

    pub fn destroy_command_encoder(&self, encoder: &mut CommandEncoder) {
        unsafe {
            self.device
                .free_command_buffers(encoder.cmd_pool, &encoder.cmd_bufs);
            self.device.destroy_command_pool(encoder.cmd_pool, None);
        };
    }

    pub fn wait_for_sync(&self) {
        let semaphores = &[self.timeline_semaphore];
        let semaphore_values = &[self.sync];
        let wait_info = vk::SemaphoreWaitInfoKHR::builder()
            .semaphores(semaphores)
            .values(semaphore_values);
        unsafe {
            self.khr_timeline_semaphore
                .wait_semaphores(&wait_info, 5_000_000_000)
                .unwrap();
        }
    }

    pub fn finish_frame(&mut self, cmd: &CommandEncoder, frame: &SwapchainFrame) {
        let cmd = cmd.finish(frame).unwrap();

        let wait_semaphores = &[frame.acquire_semaphore];

        self.sync += 1;
        let sync = self.sync;
        let wait_values_all = &[0];
        let signal_values_all = &[sync, 0];

        let mut timeline_info = vk::TimelineSemaphoreSubmitInfo::builder()
            .wait_semaphore_values(wait_values_all)
            .signal_semaphore_values(signal_values_all);

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::ALL_COMMANDS])
            .signal_semaphores(&[self.timeline_semaphore, frame.present_semaphore])
            .command_buffers(&[cmd])
            .push_next(&mut timeline_info)
            .build();

        unsafe {
            self.device
                .queue_submit(self.queue, &[submit_info], vk::Fence::null())
                .unwrap();
        }
    }
}

pub struct SwapchainFrame {
    khr_swapchain: khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    queue: vk::Queue,

    image_index: u32,
    image: vk::Image,
    view: vk::ImageView,
    format: vk::Format,
    acquire_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
    target_size: [u16; 2],
}

impl SwapchainFrame {
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
        let target_size = [configuration.width as u16, configuration.height as u16];
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
                target_size,
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

pub struct CommandEncoder {
    device: ash::Device,
    cmd_pool: vk::CommandPool,
    cmd_bufs: Vec<vk::CommandBuffer>,
}

impl CommandEncoder {
    fn current_command_buffer(&self) -> vk::CommandBuffer {
        self.cmd_bufs[0]
    }

    pub fn begin(&mut self, frame: &SwapchainFrame) {
        self.cmd_bufs.rotate_left(1);

        let vk_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        unsafe {
            self.device
                .begin_command_buffer(self.current_command_buffer(), &vk_info)
                .unwrap();
        }

        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::GENERAL)
            .image(frame.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            })
            .build();
        unsafe {
            self.device.cmd_pipeline_barrier(
                self.current_command_buffer(),
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    pub fn finish(&self, frame: &SwapchainFrame) -> Result<vk::CommandBuffer> {
        unsafe {
            let barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .image(frame.image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build();

            self.device.cmd_pipeline_barrier(
                self.current_command_buffer(),
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            self.device
                .end_command_buffer(self.current_command_buffer())?;
        }

        Ok(self.current_command_buffer())
    }
}

pub struct CommandBuffer {
    buf: vk::CommandBuffer,
}

impl CommandBuffer {}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use std::borrow::Cow;

    let callback_data = *p_callback_data;

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => tracing::error!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => tracing::warn!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => tracing::info!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => tracing::debug!("{message}"),
        _ => tracing::error!("(unknown level) {message}"),
    };

    vk::FALSE
}
