use anyhow::Result;
use ash::vk;

use crate::gapi::surface::SwapchainFrame;

pub struct CommandEncoder {
    device: ash::Device,
    pub(super) cmd_pool: vk::CommandPool,
    pub(super) cmd_bufs: Vec<vk::CommandBuffer>,
}

impl CommandEncoder {
    pub(super) fn new(device: &ash::Device, queue_index: u32, buffer_count: u32) -> Result<Self> {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let cmd_pool = unsafe { device.create_command_pool(&create_info, None)? };

        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(buffer_count)
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let cmd_bufs = unsafe { device.allocate_command_buffers(&allocate_info)? };

        Ok(CommandEncoder {
            device: device.clone(),
            cmd_pool,
            cmd_bufs,
        })
    }

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
