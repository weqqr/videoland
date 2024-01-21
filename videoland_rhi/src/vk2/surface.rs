use std::sync::Arc;

use ash::extensions::khr;
use ash::vk;

use crate::vk2::Instance;

use super::Error;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub struct Surface {
    instance: Arc<Instance>,

    surface_ext: khr::Surface,
    surface: vk::SurfaceKHR,
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_ext.destroy_surface(self.surface, None);
        }
    }
}

impl Surface {
    pub(super) unsafe fn new<W>(instance: Arc<Instance>, window: W) -> Result<Self, Error>
    where
        W: HasWindowHandle,
    {
        let surface_ext = khr::Surface::new(instance.entry(), instance.raw());

        let surface = Self::create_surface(
            instance.entry(),
            instance.raw(),
            window.window_handle().unwrap().as_raw(),
        )?;

        Ok(Self {
            instance,

            surface_ext,

            surface,
        })
    }

    #[cfg(windows)]
    unsafe fn create_surface(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window_handle: RawWindowHandle,
    ) -> Result<vk::SurfaceKHR, Error> {
        let raw_window_handle::RawWindowHandle::Win32(handle) = window_handle else {
            panic!("unexpected window handle");
        };

        let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
            .hinstance(
                handle
                    .hinstance
                    .map(|hinstance| hinstance.get() as *const std::ffi::c_void)
                    .unwrap_or(std::ptr::null()),
            )
            .hwnd(handle.hwnd.get() as *const std::ffi::c_void);

        let win32_surface_ext = khr::Win32Surface::new(entry, instance);
        Ok(win32_surface_ext.create_win32_surface(&create_info, None)?)
    }

    pub(super) fn raw(&self) -> vk::SurfaceKHR {
        self.surface
    }

    pub(super) fn ext(&self) -> khr::Surface {
        self.surface_ext.clone()
    }
}
