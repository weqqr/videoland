use std::ffi::CStr;

use ash::extensions::{ext, khr};
use ash::vk;

use crate::vk2::Surface;

use super::Error;

pub(super) struct Instance {
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

#[cfg(windows)]
const REQUIRED_SURFACE_EXTENSIONS: &[*const std::ffi::c_char] = &[
    khr::Surface::name().as_ptr(),
    khr::Win32Surface::name().as_ptr(),
];

impl Instance {
    pub(super) unsafe fn new() -> Result<Self, Error> {
        let entry = ash::Entry::load()?;

        let khronos_validation =
            CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap();
        let layers = vec![khronos_validation.as_ptr()];

        let mut extensions = vec![ext::DebugUtils::name().as_ptr()];
        extensions.extend_from_slice(REQUIRED_SURFACE_EXTENSIONS);

        let application_info = vk::ApplicationInfo::builder()
            .api_version(vk::API_VERSION_1_3)
            .engine_name(CStr::from_bytes_with_nul(b"videoland\0").unwrap())
            .engine_version(2);

        let create_info = vk::InstanceCreateInfo::builder()
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers)
            .application_info(&application_info);

        let instance = entry.create_instance(&create_info, None)?;

        let debug_utils = ext::DebugUtils::new(&entry, &instance);

        let severity = vk::DebugUtilsMessageSeverityFlagsEXT::empty()
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

        let debug_utils_messenger = debug_utils.create_debug_utils_messenger(&create_info, None)?;

        Ok(Self {
            entry,
            instance,
            debug_utils,
            debug_utils_messenger,
        })
    }

    pub(super) unsafe fn get_physical_device(
        &self,
        surface: &Surface,
    ) -> Result<PhysicalDevice, Error> {
        let devices = self.instance.enumerate_physical_devices().unwrap();

        let mut selected_device = None;

        for device in devices.iter().cloned() {
            let device =
                PhysicalDevice::new(&self.instance, device, &surface.ext(), surface.raw())?;

            selected_device = Some(device);
        }

        selected_device.ok_or_else(|| Error::NoDevices)
    }

    pub(super) fn raw(&self) -> &ash::Instance {
        &self.instance
    }

    pub(super) fn entry(&self) -> &ash::Entry {
        &self.entry
    }
}

#[derive(Clone)]
pub struct PhysicalDevice {
    pub(super) device: vk::PhysicalDevice,
    pub(super) name: String,
    pub(super) properties: vk::PhysicalDeviceProperties,
    pub(super) graphics_queue_family: u32,
}

impl PhysicalDevice {
    unsafe fn new(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
        surface_ext: &khr::Surface,
        surface: vk::SurfaceKHR,
    ) -> Result<Self, Error> {
        let properties = instance.get_physical_device_properties(device);

        let name = bytemuck::cast_slice(&properties.device_name);
        let name = CStr::from_bytes_until_nul(name).unwrap().to_owned();
        let name = name.into_string().unwrap();

        let queue_properties = instance.get_physical_device_queue_family_properties(device);

        let graphics_queue_family = queue_properties
            .iter()
            .enumerate()
            .find_map(|(index, family)| {
                let index = index as u32;

                let supports_surface = surface_ext
                    .get_physical_device_surface_support(device, index, surface)
                    .is_ok_and(|x| x);
                let is_graphics = family.queue_flags.contains(vk::QueueFlags::GRAPHICS);

                (supports_surface && is_graphics).then_some(index)
            })
            .ok_or(Error::NoDevices)?;

        Ok(PhysicalDevice {
            device,
            name,
            properties,
            graphics_queue_family,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn raw(&self) -> vk::PhysicalDevice {
        self.device
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
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => tracing::debug!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => tracing::debug!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => tracing::debug!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => tracing::debug!("{message}"),
        _ => println!("(unknown level) {message}"),
    };

    vk::FALSE
}
