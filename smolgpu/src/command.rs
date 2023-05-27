use anyhow::Result;
use ash::extensions::khr;
use ash::vk;

use crate::pipeline::Pipeline;
use crate::surface::SwapchainFrame;
use crate::{Buffer, ImageView};

pub struct CommandEncoder {
    device: ash::Device,
    khr_dynamic_rendering: khr::DynamicRendering,

    pub(super) cmd_pool: vk::CommandPool,
    pub(super) cmd_bufs: Vec<vk::CommandBuffer>,
}

impl CommandEncoder {
    pub(super) fn new(
        device: &ash::Device,
        khr_dynamic_rendering: khr::DynamicRendering,
        queue_index: u32,
        buffer_count: u32,
    ) -> Result<Self> {
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
            khr_dynamic_rendering,
            cmd_pool,
            cmd_bufs,
        })
    }

    pub fn current_command_buffer(&self) -> vk::CommandBuffer {
        self.cmd_bufs[0]
    }

    pub fn begin_rendering(&self, image_view: ImageView) {
        let color_attachments = &[vk::RenderingAttachmentInfo::builder()
            .image_view(image_view.image_view)
            .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL_KHR)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [1.0, 1.0, 1.0, 1.0],
                },
            })
            .build()];

        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: image_view.width(),
                height: image_view.height(),
            },
        };

        let rendering_info = vk::RenderingInfoKHR::builder()
            .color_attachments(color_attachments)
            .render_area(render_area)
            .layer_count(1);

        unsafe {
            self.khr_dynamic_rendering
                .cmd_begin_rendering(self.current_command_buffer(), &rendering_info);
        }
    }

    pub fn end_rendering(&self) {
        unsafe {
            self.khr_dynamic_rendering
                .cmd_end_rendering(self.current_command_buffer());
        }
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

    pub fn finish(&self, frame: &SwapchainFrame) -> Result<CommandBuffer> {
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

        Ok(CommandBuffer {
            buffer: self.current_command_buffer(),
        })
    }

    pub fn begin_immediate(&mut self) -> Result<CommandBuffer> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let cmd_buffer = unsafe { self.device.allocate_command_buffers(&allocate_info)?[0] };

        let vk_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        unsafe {
            self.device
                .begin_command_buffer(cmd_buffer, &vk_info)
                .unwrap();
        }

        Ok(CommandBuffer { buffer: cmd_buffer })
    }

    pub fn finish_immediate(&mut self, cmd_buffer: CommandBuffer) -> Result<CommandBuffer> {
        unsafe {
            self.device.end_command_buffer(cmd_buffer.buffer)?;
        }

        Ok(cmd_buffer)
    }

    pub fn bind_pipeline(&self, pipeline: &Pipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(
                self.current_command_buffer(),
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline,
            );
        }
    }

    pub fn bind_vertex_buffer(&self, buf: &Buffer) {
        unsafe {
            self.device.cmd_bind_vertex_buffers(
                self.current_command_buffer(),
                0,
                &[buf.buffer],
                &[0],
            );
        }
    }

    pub fn draw(&self, vertex_count: u32) {
        unsafe {
            self.device
                .cmd_draw(self.current_command_buffer(), vertex_count, 1, 0, 0);
        }
    }

    pub fn set_push_constants(&self, pipeline: &Pipeline, data: &[u8]) {
        unsafe {
            self.device.cmd_push_constants(
                self.current_command_buffer(),
                pipeline.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                data,
            );
        }
    }

    pub fn set_viewport(&self, width: u32, height: u32) {
        unsafe {
            self.device.cmd_set_viewport(
                self.current_command_buffer(),
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: width as f32,
                    height: height as f32,
                    min_depth: 0.0,
                    max_depth: 0.0,
                }],
            );

            self.device.cmd_set_scissor(
                self.current_command_buffer(),
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D { width, height },
                }],
            )
        }
    }
}

pub struct CommandBuffer {
    pub(super) buffer: vk::CommandBuffer,
}
