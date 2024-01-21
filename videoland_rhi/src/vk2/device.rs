use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use ash::extensions::khr;
use ash::vk;

use crate::vk2::{Instance, PhysicalDevice};
use crate::{CommandBuffer, Swapchain, SwapchainFrame};

use super::Error;

pub type MemAllocator = Arc<RwLock<gpu_alloc::GpuAllocator<vk::DeviceMemory>>>;

pub struct Device {
    instance: Arc<Instance>,

    physical_device: PhysicalDevice,
    device: ash::Device,

    dynamic_rendering_ext: khr::DynamicRendering,
    timeline_semaphore_ext: khr::TimelineSemaphore,

    timeline_semaphore: vk::Semaphore,
    sync: AtomicU64,

    queue: vk::Queue,

    allocator: MemAllocator,
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.timeline_semaphore, None);
            self.device.destroy_device(None);
        }
    }
}

impl Device {
    pub(super) unsafe fn new(
        instance: Arc<Instance>,
        physical_device: PhysicalDevice,
    ) -> Result<Self, Error> {
        let create_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(physical_device.graphics_queue_family)
            .queue_priorities(&[1.0])
            .build();

        let queue_create_infos = &[create_info];

        let extensions = vec![
            vk::KhrDynamicRenderingFn::name().as_ptr(),
            vk::KhrSwapchainFn::name().as_ptr(),
            vk::KhrTimelineSemaphoreFn::name().as_ptr(),
            vk::KhrBufferDeviceAddressFn::name().as_ptr(),
        ];

        let mut buffer_device_address = vk::PhysicalDeviceBufferDeviceAddressFeatures::builder()
            .buffer_device_address(true)
            .build();

        let mut physical_device_features = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut buffer_device_address)
            .build();

        let mut khr_dynamic_rendering =
            vk::PhysicalDeviceDynamicRenderingFeaturesKHR::builder().dynamic_rendering(true);
        let mut khr_timeline_semaphore =
            vk::PhysicalDeviceTimelineSemaphoreFeaturesKHR::builder().timeline_semaphore(true);
        let create_info = vk::DeviceCreateInfo::builder()
            .enabled_extension_names(&extensions)
            .queue_create_infos(queue_create_infos)
            .push_next(&mut khr_dynamic_rendering)
            .push_next(&mut khr_timeline_semaphore)
            .push_next(&mut physical_device_features);

        let device = instance
            .raw()
            .create_device(physical_device.device, &create_info, None)?;

        let khr_dynamic_rendering_ext = khr::DynamicRendering::new(instance.raw(), &device);
        let khr_timeline_semaphore_ext = khr::TimelineSemaphore::new(instance.raw(), &device);

        let queue = device.get_device_queue(physical_device.graphics_queue_family, 0);

        let mut semaphore_type_create_info = vk::SemaphoreTypeCreateInfo::builder()
            .semaphore_type(vk::SemaphoreType::TIMELINE_KHR)
            .initial_value(0);
        let create_info =
            vk::SemaphoreCreateInfo::builder().push_next(&mut semaphore_type_create_info);
        let timeline_semaphore = device.create_semaphore(&create_info, None)?;

        let properties = unsafe {
            gpu_alloc_ash::device_properties(
                instance.raw(),
                vk::API_VERSION_1_3,
                physical_device.device,
            )?
        };

        let allocator = Arc::new(RwLock::new(gpu_alloc::GpuAllocator::new(
            gpu_alloc::Config::i_am_prototyping(),
            properties,
        )));

        Ok(Device {
            instance,

            physical_device: physical_device.clone(),
            device,

            dynamic_rendering_ext: khr_dynamic_rendering_ext,
            timeline_semaphore_ext: khr_timeline_semaphore_ext,

            timeline_semaphore,
            sync: AtomicU64::new(0),

            queue,

            allocator,
        })
    }

    pub(super) unsafe fn submit_frame(
        &self,
        command_buffer: CommandBuffer,
        swapchain: &Swapchain,
        frame: &SwapchainFrame,
    ) -> Result<(), Error> {
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .image(frame.raw_image())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();

        self.device.cmd_pipeline_barrier(
            command_buffer.raw(),
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        self.device.end_command_buffer(command_buffer.raw())?;

        self.sync.fetch_add(1, Ordering::SeqCst);

        let wait_semaphores = &[frame.acquire_semaphore];
        let signal_semaphores = &[self.timeline_semaphore, frame.present_semaphore];

        let wait_values = &[0];
        let signal_values = &[self.sync.load(Ordering::SeqCst), 0];

        let mut timeline_info = vk::TimelineSemaphoreSubmitInfo::builder()
            .wait_semaphore_values(wait_values)
            .signal_semaphore_values(signal_values);

        let mask = &[vk::PipelineStageFlags::ALL_COMMANDS];
        let command_buffers = &[command_buffer.raw()];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(mask)
            .signal_semaphores(signal_semaphores)
            .command_buffers(command_buffers)
            .push_next(&mut timeline_info)
            .build();

        self.device
            .queue_submit(self.queue, &[submit_info], vk::Fence::null())
            .unwrap();

        let wait_semaphores = &[frame.present_semaphore];
        let swapchains = &[swapchain.raw()];
        let image_indices = &[frame.index];

        let present_info = vk::PresentInfoKHR::builder()
            .swapchains(swapchains)
            .wait_semaphores(wait_semaphores)
            .image_indices(image_indices);

        swapchain
            .ext()
            .queue_present(self.queue, &present_info)
            .unwrap();

        self.wait_for_sync();

        Ok(())
    }

    pub fn wait_for_sync(&self) {
        let semaphores = &[self.timeline_semaphore];
        let semaphore_values = &[self.sync.load(Ordering::SeqCst)];
        let wait_info = vk::SemaphoreWaitInfoKHR::builder()
            .semaphores(semaphores)
            .values(semaphore_values);

        unsafe {
            self.timeline_semaphore_ext
                .wait_semaphores(&wait_info, 5_000_000_000)
                .unwrap();
        }
    }

    pub(super) fn raw(&self) -> &ash::Device {
        &self.device
    }

    pub(super) fn physical(&self) -> &PhysicalDevice {
        &self.physical_device
    }

    pub(super) fn dynamic_rendering_ext(&self) -> &khr::DynamicRendering {
        &self.dynamic_rendering_ext
    }

    pub(super) fn timeline_semaphore_ext(&self) -> &khr::TimelineSemaphore {
        &self.timeline_semaphore_ext
    }

    pub(super) fn allocator(&self) -> MemAllocator {
        Arc::clone(&self.allocator)
    }
}
