use std::ffi::{c_char, CStr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use ash::extensions::{ext, khr};
use ash::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use tinyvec::ArrayVec;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Vulkan instance has no supported devices")]
    NoDevices,

    #[error("Vulkan error: {0}")]
    Vulkan(#[from] ash::vk::Result),

    #[error("Vulkan loading error: {0}")]
    Loading(#[from] ash::LoadingError),

    #[error("memory allocation error: {0}")]
    Allocation(#[from] gpu_alloc::AllocationError),
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

#[cfg(windows)]
const REQUIRED_SURFACE_EXTENSIONS: &[*const c_char] = &[
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
            .engine_version(1);
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
            let device = PhysicalDevice::new(
                &self.instance,
                device,
                &surface.surface_ext,
                surface.surface,
            )?;

            selected_device = Some(device);
        }

        selected_device.ok_or_else(|| Error::NoDevices)
    }

    pub(super) unsafe fn create_surface<W>(&self, window: W) -> Result<Surface, Error>
    where
        W: HasRawDisplayHandle + HasRawWindowHandle,
    {
        Surface::new(&self.entry, &self.instance, window)
    }

    pub(super) unsafe fn create_device(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Result<Device, Error> {
        Device::new(&self.instance, physical_device)
    }
}

#[derive(Clone)]
pub struct PhysicalDevice {
    device: vk::PhysicalDevice,
    name: String,
    properties: vk::PhysicalDeviceProperties,
    graphics_queue_family: u32,
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
}

pub struct Surface {
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
    unsafe fn new<W>(entry: &ash::Entry, instance: &ash::Instance, window: W) -> Result<Self, Error>
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
        let display_handle = window.raw_display_handle();
        let window_handle = window.raw_window_handle();

        let surface_ext = khr::Surface::new(entry, instance);

        let surface =
            ash_window::create_surface(entry, instance, display_handle, window_handle, None)?;

        Ok(Self {
            surface_ext,

            surface,
        })
    }
}

pub struct Device {
    physical_device: PhysicalDevice,
    device: ash::Device,
    instance: ash::Instance,

    dynamic_rendering_ext: khr::DynamicRendering,
    timeline_semaphore_ext: khr::TimelineSemaphore,

    timeline_semaphore: vk::Semaphore,
    sync: AtomicU64,

    queue: vk::Queue,

    allocator: Arc<RwLock<gpu_alloc::GpuAllocator<vk::DeviceMemory>>>,
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
    pub unsafe fn new(
        instance: &ash::Instance,
        physical_device: &PhysicalDevice,
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

        let device = instance.create_device(physical_device.device, &create_info, None)?;

        let khr_dynamic_rendering_ext = khr::DynamicRendering::new(instance, &device);
        let khr_timeline_semaphore_ext = khr::TimelineSemaphore::new(instance, &device);

        let queue = device.get_device_queue(physical_device.graphics_queue_family, 0);

        let mut semaphore_type_create_info = vk::SemaphoreTypeCreateInfo::builder()
            .semaphore_type(vk::SemaphoreType::TIMELINE_KHR)
            .initial_value(0);
        let create_info =
            vk::SemaphoreCreateInfo::builder().push_next(&mut semaphore_type_create_info);
        let timeline_semaphore = device.create_semaphore(&create_info, None)?;

        let properties = unsafe {
            gpu_alloc_ash::device_properties(instance, vk::API_VERSION_1_3, physical_device.device)?
        };

        let allocator = Arc::new(RwLock::new(gpu_alloc::GpuAllocator::new(
            gpu_alloc::Config::i_am_prototyping(),
            properties,
        )));

        Ok(Device {
            physical_device: physical_device.clone(),
            device,
            instance: instance.clone(),

            dynamic_rendering_ext: khr_dynamic_rendering_ext,
            timeline_semaphore_ext: khr_timeline_semaphore_ext,

            timeline_semaphore,
            sync: AtomicU64::new(0),

            queue,

            allocator,
        })
    }

    pub(super) unsafe fn create_swapchain(&self, surface: &Surface) -> Result<Swapchain, Error> {
        Swapchain::new(&self.instance, self, surface)
    }

    pub(super) unsafe fn create_command_encoder(&self) -> Result<CommandEncoder, Error> {
        CommandEncoder::new(
            &self.device,
            self.dynamic_rendering_ext.clone(),
            self.physical_device.graphics_queue_family,
            2,
        )
    }

    pub(super) unsafe fn create_shader_module(&self, spirv: &[u32]) -> Result<ShaderModule, Error> {
        ShaderModule::new(&self.device, spirv)
    }

    pub(super) unsafe fn destroy_shader_module(&self, shader_module: ShaderModule) {
        unsafe {
            self.device
                .destroy_shader_module(shader_module.shader_module, None);
        }
    }

    pub(super) unsafe fn create_pipeline(
        &self,
        desc: &super::PipelineDesc,
    ) -> Result<Pipeline, Error> {
        Pipeline::new(&self.device, desc)
    }

    pub(super) unsafe fn destroy_pipeline(&self, pipeline: Pipeline) {
        unsafe {
            self.device.destroy_pipeline(pipeline.pipeline, None);
            self.device
                .destroy_pipeline_layout(pipeline.pipeline_layout, None);
        }
    }

    pub(super) unsafe fn create_buffer(
        &self,
        allocation: super::BufferAllocation,
    ) -> Result<Buffer, Error> {
        Buffer::new(&self.device, Arc::clone(&self.allocator), allocation)
    }

    pub(super) unsafe fn destroy_buffer(&self, mut buffer: Buffer) {
        let allocation = buffer.allocation.take().unwrap();

        unsafe {
            self.allocator.write().unwrap().dealloc(
                gpu_alloc_ash::AshMemoryDevice::wrap(&self.device),
                allocation,
            );

            self.device.destroy_buffer(buffer.buffer, None);
        }
    }

    pub(super) unsafe fn create_texture(
        &self,
        command_buffer: CommandBuffer,
        desc: &super::TextureDesc,
    ) -> Result<Texture, Error> {
        Texture::new(
            &self.device,
            command_buffer.command_buffer,
            Arc::clone(&self.allocator),
            desc,
        )
    }

    pub(super) unsafe fn destroy_texture(&self, texture: &mut Texture) {
        let allocation = texture.allocation.take().unwrap();

        unsafe {
            self.allocator.write().unwrap().dealloc(
                gpu_alloc_ash::AshMemoryDevice::wrap(&self.device),
                allocation,
            );

            self.device.destroy_image(texture.image, None);
        }
    }

    pub(super) unsafe fn create_texture_view(
        &self,
        texture: &Texture,
        desc: &super::TextureViewDesc,
    ) -> Result<TextureView, Error> {
        TextureView::new(&self.device, texture, desc)
    }

    pub(super) unsafe fn destroy_texture_view(&self, texture_view: &mut TextureView) {
        unsafe {
            self.device
                .destroy_image_view(texture_view.image_view, None);
        }
    }

    pub unsafe fn wait_for_sync(&self) {
        let semaphores = &[self.timeline_semaphore];
        let semaphore_values = &[self.sync.load(Ordering::SeqCst)];
        let wait_info = vk::SemaphoreWaitInfoKHR::builder()
            .semaphores(semaphores)
            .values(semaphore_values);

        self.timeline_semaphore_ext
            .wait_semaphores(&wait_info, 5_000_000_000)
            .unwrap();
    }

    pub(super) unsafe fn submit_frame(
        &self,
        _command_encoder: &CommandEncoder,
        command_buffer: CommandBuffer,
        swapchain: &Swapchain,
        frame: &SwapchainFrame,
    ) -> Result<(), Error> {
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
            command_buffer.command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        self.device
            .end_command_buffer(command_buffer.command_buffer)?;

        self.sync.fetch_add(1, Ordering::SeqCst);

        let wait_semaphores = &[frame.acquire_semaphore];
        let signal_semaphores = &[self.timeline_semaphore, frame.present_semaphore];

        let wait_values = &[0];
        let signal_values = &[self.sync.load(Ordering::SeqCst), 0];

        let mut timeline_info = vk::TimelineSemaphoreSubmitInfo::builder()
            .wait_semaphore_values(wait_values)
            .signal_semaphore_values(signal_values);

        let mask = &[vk::PipelineStageFlags::ALL_COMMANDS];
        let command_buffers = &[command_buffer.command_buffer];

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
        let swapchains = &[swapchain.swapchain];
        let image_indices = &[frame.index];

        let present_info = vk::PresentInfoKHR::builder()
            .swapchains(swapchains)
            .wait_semaphores(wait_semaphores)
            .image_indices(image_indices);

        swapchain
            .swapchain_ext
            .queue_present(self.queue, &present_info)
            .unwrap();

        self.wait_for_sync();

        Ok(())
    }
}

pub struct Swapchain {
    device: ash::Device,
    physical_device: PhysicalDevice,
    swapchain_ext: khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    surface_ext: khr::Surface,
    surface: vk::SurfaceKHR,

    frames: Vec<SwapchainFrame>,

    next_frame_semaphore: vk::Semaphore,
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for frame in self.frames.drain(..) {
                self.device.destroy_image_view(frame.view.image_view, None);
                self.device.destroy_semaphore(frame.acquire_semaphore, None);
                self.device.destroy_semaphore(frame.present_semaphore, None);
            }

            self.device
                .destroy_semaphore(self.next_frame_semaphore, None);

            if self.swapchain != vk::SwapchainKHR::null() {
                self.swapchain_ext.destroy_swapchain(self.swapchain, None);
            }
        }
    }
}

impl Swapchain {
    pub(super) unsafe fn new(
        instance: &ash::Instance,
        device: &Device,
        surface: &Surface,
    ) -> Result<Self, Error> {
        let swapchain_ext = khr::Swapchain::new(instance, &device.device);

        let swapchain = vk::SwapchainKHR::null();

        let next_frame_semaphore = device
            .device
            .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)
            .unwrap();

        Ok(Self {
            device: device.device.clone(),
            physical_device: device.physical_device.clone(),
            swapchain_ext,
            swapchain,
            surface_ext: surface.surface_ext.clone(),
            surface: surface.surface,

            frames: Vec::new(),

            next_frame_semaphore,
        })
    }

    pub(super) unsafe fn resize(&mut self, size: super::Extent2D) -> Result<(), Error> {
        let surface_format = self
            .surface_ext
            .get_physical_device_surface_formats(self.physical_device.device, self.surface)?[0];

        let surface_capabilities = self
            .surface_ext
            .get_physical_device_surface_capabilities(self.physical_device.device, self.surface)?;

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
            .surface(self.surface)
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
            self.device.destroy_image_view(frame.view.image_view, None);
            self.device.destroy_semaphore(frame.acquire_semaphore, None);
            self.device.destroy_semaphore(frame.present_semaphore, None);
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
                .create_image_view(&view_create_info, None)
                .unwrap();

            let view = TextureView {
                image_view: view,
                width: size.width,
                height: size.height,
            };

            let acquire_semaphore = self
                .device
                .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)
                .unwrap();

            let present_semaphore = self
                .device
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

    pub unsafe fn acquire_next_frame(&mut self) -> SwapchainFrame {
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
}

#[derive(Clone)]
pub struct SwapchainFrame {
    image: vk::Image,
    view: TextureView,
    index: u32,
    acquire_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
}

impl SwapchainFrame {
    pub fn image_view(&self) -> TextureView {
        self.view
    }
}

#[derive(Clone, Copy)]
pub struct TextureView {
    image_view: vk::ImageView,
    width: u32,
    height: u32,
}

impl TextureView {
    unsafe fn new(
        device: &ash::Device,
        texture: &Texture,
        desc: &super::TextureViewDesc,
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

        let image_view = device.create_image_view(&create_info, None)?;

        Ok(Self {
            image_view,
            width: desc.extent.width,
            height: desc.extent.height,
        })
    }

    pub(super) fn width(&self) -> u32 {
        self.width
    }

    pub(super) fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Clone)]
pub struct CommandBuffer {
    device: ash::Device,
    khr_dynamic_rendering: khr::DynamicRendering,

    command_buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    pub(super) unsafe fn begin(&self) {
        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        self.device
            .begin_command_buffer(self.command_buffer, &info)
            .unwrap();
    }

    pub(super) unsafe fn begin_rendering(&self, desc: super::RenderPassDesc) {
        let color_attachments = &[vk::RenderingAttachmentInfo::builder()
            .image_view(desc.color_attachment.texture_view.image_view)
            .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL_KHR)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.1, 0.2, 0.3, 1.0],
                },
            })
            .build()];

        let depth_attachment = vk::RenderingAttachmentInfo::builder()
            .image_view(desc.depth_attachment.texture_view.image_view)
            .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL_KHR)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue::builder().depth(1.0).build(),
            });

        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: desc.render_area.width,
                height: desc.render_area.height,
            },
        };

        let rendering_info = vk::RenderingInfoKHR::builder()
            .color_attachments(color_attachments)
            .depth_attachment(&depth_attachment)
            .render_area(render_area)
            .layer_count(1);

        self.khr_dynamic_rendering
            .cmd_begin_rendering(self.command_buffer, &rendering_info);
    }

    pub(super) unsafe fn texture_barrier(
        &self,
        texture: &Texture,
        old: super::TextureLayout,
        new: super::TextureLayout,
    ) {
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(old.into())
            .new_layout(new.into())
            .image(texture.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            })
            .build();

        self.device.cmd_pipeline_barrier(
            self.command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }

    pub(super) unsafe fn end_rendering(&self) {
        self.khr_dynamic_rendering
            .cmd_end_rendering(self.command_buffer);
    }

    pub(super) unsafe fn set_viewport(&self, extent: super::Extent2D) {
        self.device.cmd_set_viewport(
            self.command_buffer,
            0,
            &[vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );

        self.device.cmd_set_scissor(
            self.command_buffer,
            0,
            &[vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: extent.width,
                    height: extent.height,
                },
            }],
        );
    }

    pub(super) unsafe fn bind_pipeline(&self, pipeline: &Pipeline) {
        self.device.cmd_bind_pipeline(
            self.command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline,
        );
    }

    pub(super) unsafe fn bind_vertex_buffer(&self, buffer: &Buffer) {
        self.device
            .cmd_bind_vertex_buffers(self.command_buffer, 0, &[buffer.buffer], &[0]);
    }

    pub(super) unsafe fn set_push_constants(
        &self,
        pipeline: &Pipeline,
        offset: u32,
        constants: &[u8],
    ) {
        let stage_flags = vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT;
        self.device.cmd_push_constants(
            self.command_buffer,
            pipeline.pipeline_layout,
            stage_flags,
            offset,
            constants,
        )
    }

    pub(super) unsafe fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        self.device.cmd_draw(
            self.command_buffer,
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        );
    }
}

pub struct CommandEncoder {
    device: ash::Device,
    khr_dynamic_rendering: khr::DynamicRendering,

    command_pool: vk::CommandPool,
    command_buffers: Vec<CommandBuffer>,
}

impl Drop for CommandEncoder {
    fn drop(&mut self) {
        let command_buffers: Vec<_> = self
            .command_buffers
            .iter()
            .map(|buffer| buffer.command_buffer)
            .collect();

        unsafe {
            self.device
                .free_command_buffers(self.command_pool, &command_buffers);
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}

impl CommandEncoder {
    unsafe fn new(
        device: &ash::Device,
        khr_dynamic_rendering: khr::DynamicRendering,
        queue_index: u32,
        buffer_count: u32,
    ) -> Result<Self, Error> {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = device.create_command_pool(&create_info, None)?;

        let mut command_encoder = CommandEncoder {
            device: device.clone(),
            khr_dynamic_rendering,
            command_pool,
            command_buffers: Vec::new(),
        };

        for _ in 0..buffer_count {
            let command_buffer = command_encoder.create_command_buffer()?;
            command_encoder.command_buffers.push(command_buffer);
        }

        Ok(command_encoder)
    }

    pub(super) unsafe fn create_command_buffer(&self) -> Result<CommandBuffer, Error> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer = self.device.allocate_command_buffers(&allocate_info)?[0];

        Ok(CommandBuffer {
            device: self.device.clone(),
            khr_dynamic_rendering: self.khr_dynamic_rendering.clone(),

            command_buffer,
        })
    }

    pub(super) fn current_command_buffer(&self) -> CommandBuffer {
        self.command_buffers[0].clone()
    }

    pub(super) unsafe fn begin(&mut self, frame: &SwapchainFrame) -> CommandBuffer {
        self.command_buffers.rotate_left(1);

        let command_buffer = self.current_command_buffer();

        command_buffer.begin();

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

        self.device.cmd_pipeline_barrier(
            command_buffer.command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        command_buffer
    }
}

pub struct ShaderModule {
    device: ash::Device,
    shader_module: vk::ShaderModule,
}

impl ShaderModule {
    pub fn new(device: &ash::Device, spirv: &[u32]) -> Result<Self, Error> {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(spirv);

        let shader_module = unsafe { device.create_shader_module(&create_info, None)? };

        Ok(Self {
            device: device.clone(),
            shader_module,
        })
    }
}

pub struct PipelineDesc<'a> {
    pub vertex_shader: &'a ShaderModule,
    pub fragment_shader: &'a ShaderModule,
}

pub struct Pipeline {
    device: ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl Pipeline {
    const MAX_VERTEX_BUFFER_ATTRIBUTES: usize = 16;

    unsafe fn new(device: &ash::Device, desc: &super::PipelineDesc) -> Result<Self, Error> {
        let vertex_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .name(CStr::from_bytes_with_nul(b"vs_main\0").unwrap())
            .module(desc.vertex.shader_module.shader_module);

        let fragment_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .name(CStr::from_bytes_with_nul(b"fs_main\0").unwrap())
            .module(desc.fragment.shader_module.shader_module);

        let shader_stages = &[
            vertex_shader_stage_create_info.build(),
            fragment_shader_stage_create_info.build(),
        ];

        let vertex_attribute_descriptions = desc
            .vertex_layout
            .attributes
            .iter()
            .map(|attr| attr.clone().into())
            .collect::<ArrayVec<[vk::VertexInputAttributeDescription; Self::MAX_VERTEX_BUFFER_ATTRIBUTES]>>();

        let vertex_binding_descriptions = &[vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(8 * 4)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_attribute_descriptions)
            .vertex_binding_descriptions(vertex_binding_descriptions);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE);

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .min_sample_shading(1.0)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false);

        let color_blend_attachments = &[color_blend_attachment_state.build()];

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::COPY)
            .logic_op_enable(false)
            .attachments(color_blend_attachments);

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&[
            vk::DynamicState::VIEWPORT,
            vk::DynamicState::SCISSOR,
            vk::DynamicState::LINE_WIDTH,
        ]);

        let mut rendering = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(&[vk::Format::B8G8R8A8_UNORM])
            .depth_attachment_format(vk::Format::D24_UNORM_S8_UINT)
            .build();

        let ranges = &[vk::PushConstantRange::builder()
            .size(256)
            .offset(0)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .build()];

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(ranges)
            .set_layouts(&[]);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None)? };

        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1);

        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);

        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .input_assembly_state(&input_assembly_state)
            .vertex_input_state(&vertex_input_state)
            .multisample_state(&multisample_state)
            .layout(pipeline_layout)
            .rasterization_state(&rasterization_state)
            .stages(shader_stages)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state)
            .viewport_state(&viewport_state)
            .depth_stencil_state(&depth_stencil_state)
            .push_next(&mut rendering);

        let pipeline = device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info.build()], None)
            .unwrap()
            .pop()
            .unwrap();

        Ok(Pipeline {
            device: device.clone(),
            pipeline,
            pipeline_layout,
        })
    }
}

type Allocator = Arc<RwLock<gpu_alloc::GpuAllocator<vk::DeviceMemory>>>;

pub struct Buffer {
    device: ash::Device,
    allocator: Arc<RwLock<gpu_alloc::GpuAllocator<vk::DeviceMemory>>>,
    allocation: Option<gpu_alloc::MemoryBlock<vk::DeviceMemory>>,
    buffer: vk::Buffer,
}

impl Buffer {
    unsafe fn new(
        device: &ash::Device,
        allocator: Arc<RwLock<gpu_alloc::GpuAllocator<vk::DeviceMemory>>>,
        allocation: super::BufferAllocation,
    ) -> Result<Self, Error> {
        let create_info = vk::BufferCreateInfo::builder()
            .size(allocation.size)
            .usage(usage_to_vk(allocation.usage));

        let buffer = device.create_buffer(&create_info, None)?;
        let requirements = device.get_buffer_memory_requirements(buffer);

        let allocation = allocator.write().unwrap().alloc(
            gpu_alloc_ash::AshMemoryDevice::wrap(device),
            gpu_alloc::Request {
                size: requirements.size,
                align_mask: requirements.alignment,
                usage: match allocation.location {
                    super::BufferLocation::Cpu => gpu_alloc::UsageFlags::UPLOAD,
                    super::BufferLocation::Gpu => gpu_alloc::UsageFlags::FAST_DEVICE_ACCESS,
                },
                memory_types: requirements.memory_type_bits,
            },
        )?;

        device.bind_buffer_memory(buffer, *allocation.memory(), allocation.offset())?;

        Ok(Self {
            device: device.clone(),
            allocator,
            buffer,
            allocation: Some(allocation),
        })
    }

    pub(super) unsafe fn write_data(&mut self, offset: u64, data: &[u8]) {
        self.allocation
            .as_mut()
            .unwrap()
            .write_bytes(
                gpu_alloc_ash::AshMemoryDevice::wrap(&self.device),
                offset,
                data,
            )
            .unwrap();
    }
}

pub struct Texture {
    allocator: Arc<RwLock<gpu_alloc::GpuAllocator<vk::DeviceMemory>>>,
    allocation: Option<gpu_alloc::MemoryBlock<vk::DeviceMemory>>,
    image: vk::Image,
}

impl Texture {
    unsafe fn new(
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        allocator: Arc<RwLock<gpu_alloc::GpuAllocator<vk::DeviceMemory>>>,
        desc: &super::TextureDesc,
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

        let image = device.create_image(&create_info, None)?;
        let requirements = device.get_image_memory_requirements(image);

        let allocation = allocator.write().unwrap().alloc(
            gpu_alloc_ash::AshMemoryDevice::wrap(device),
            gpu_alloc::Request {
                size: requirements.size,
                align_mask: requirements.alignment,
                usage: gpu_alloc::UsageFlags::FAST_DEVICE_ACCESS,
                memory_types: requirements.memory_type_bits,
            },
        )?;

        device.bind_image_memory(image, *allocation.memory(), allocation.offset())?;

        Ok(Self {
            allocator,
            allocation: Some(allocation),
            image,
        })
    }
}

fn usage_to_vk(usage: super::BufferUsage) -> vk::BufferUsageFlags {
    let mut vk_usage = vk::BufferUsageFlags::empty();

    if usage.contains(super::BufferUsage::VERTEX) {
        vk_usage |= vk::BufferUsageFlags::VERTEX_BUFFER;
    }

    vk_usage
}

impl From<super::TextureLayout> for vk::ImageLayout {
    fn from(value: super::TextureLayout) -> Self {
        match value {
            super::TextureLayout::Undefined => vk::ImageLayout::UNDEFINED,
            super::TextureLayout::General => vk::ImageLayout::GENERAL,
            super::TextureLayout::Color => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            super::TextureLayout::DepthStencil => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            super::TextureLayout::TransferSrc => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            super::TextureLayout::TransferDst => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        }
    }
}

impl From<super::VertexFormat> for vk::Format {
    fn from(value: super::VertexFormat) -> Self {
        match value {
            super::VertexFormat::Float32x1 => vk::Format::R32_SFLOAT,
            super::VertexFormat::Float32x2 => vk::Format::R32G32_SFLOAT,
            super::VertexFormat::Float32x3 => vk::Format::R32G32B32_SFLOAT,
            super::VertexFormat::Float32x4 => vk::Format::R32G32B32A32_SFLOAT,
        }
    }
}

impl From<super::VertexAttribute> for vk::VertexInputAttributeDescription {
    fn from(value: super::VertexAttribute) -> Self {
        vk::VertexInputAttributeDescription::builder()
            .binding(value.binding)
            .location(value.location)
            .offset(value.offset)
            .format(value.format.into())
            .build()
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
