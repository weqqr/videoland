use std::ffi::CStr;

use anyhow::{anyhow, Context, Result};
use ash::extensions::{ext, khr};
use ash::vk::{self, DebugUtilsMessageSeverityFlagsEXT};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::gapi::command::CommandEncoder;
use crate::gapi::surface::{Surface, SwapchainFrame};

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
        Surface::new(
            &self.entry,
            &self.instance,
            &device.device,
            device.queue,
            display_handle,
            window_handle,
        )
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
        CommandEncoder::new(&self.device, self.graphics_queue_family_index, frames)
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
