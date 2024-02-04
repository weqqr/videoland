use std::sync::Arc;

use ash::vk;

use crate::{Buffer, Device, Error, Pipeline, SwapchainFrame, Texture};

pub struct CommandEncoder {
    device: Arc<Device>,

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
                .raw()
                .free_command_buffers(self.command_pool, &command_buffers);
            self.device
                .raw()
                .destroy_command_pool(self.command_pool, None);
        }
    }
}

impl CommandEncoder {
    pub(super) unsafe fn new(
        device: Arc<Device>,
        queue_family_index: u32,
        buffer_count: u32,
    ) -> Result<Self, Error> {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = device.raw().create_command_pool(&create_info, None)?;

        let mut command_encoder = CommandEncoder {
            device: device.clone(),
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

        let command_buffer = self.device.raw().allocate_command_buffers(&allocate_info)?[0];

        Ok(CommandBuffer {
            device: Arc::clone(&self.device),

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
            .image(frame.raw_image())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            })
            .build();

        self.device.raw().cmd_pipeline_barrier(
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

#[derive(Clone)]
pub struct CommandBuffer {
    device: Arc<Device>,

    command_buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    pub(super) unsafe fn begin(&self) {
        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        self.device
            .raw()
            .begin_command_buffer(self.command_buffer, &info)
            .unwrap();
    }

    pub fn begin_rendering(&self, desc: crate::RenderPassDesc) {
        let color_attachments = &[vk::RenderingAttachmentInfo::builder()
            .image_view(desc.color_attachment.raw())
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
            .image_view(desc.depth_attachment.raw())
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

        unsafe {
            self.device
                .dynamic_rendering_ext()
                .cmd_begin_rendering(self.command_buffer, &rendering_info);
        }
    }

    pub fn texture_barrier(
        &self,
        texture: &Texture,
        old: crate::TextureLayout,
        new: crate::TextureLayout,
    ) {
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(old.into())
            .new_layout(new.into())
            .image(texture.raw())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            })
            .build();

        unsafe {
            self.device.raw().cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    pub fn end_rendering(&self) {
        unsafe {
            self.device
                .dynamic_rendering_ext()
                .cmd_end_rendering(self.command_buffer);
        }
    }

    pub fn set_viewport(&self, extent: crate::Extent2D) {
        unsafe {
            self.device.raw().cmd_set_viewport(
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

            self.device.raw().cmd_set_scissor(
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
    }

    pub fn bind_pipeline(&self, pipeline: &Pipeline) {
        unsafe {
            self.device.raw().cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.raw(),
            );
        }
    }

    pub fn bind_vertex_buffer(&self, buffer: &Buffer) {
        unsafe {
            self.device.raw().cmd_bind_vertex_buffers(
                self.command_buffer,
                0,
                &[buffer.raw()],
                &[0],
            );
        }
    }

    pub fn bind_index_buffer(&self, buffer: &Buffer) {
        unsafe {
            self.device.raw().cmd_bind_index_buffer(
                self.command_buffer,
                buffer.raw(),
                0,
                vk::IndexType::UINT32,
            );
        }
    }

    pub fn set_push_constants(&self, pipeline: &Pipeline, offset: u32, constants: &[u8]) {
        let stage_flags = vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT;
        unsafe {
            self.device.raw().cmd_push_constants(
                self.command_buffer,
                pipeline.raw_layout(),
                stage_flags,
                offset,
                constants,
            );
        }
    }

    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.raw().cmd_draw(
                self.command_buffer,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
        }
    }

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.raw().cmd_draw_indexed(
                self.command_buffer,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }

    pub fn copy_buffer_to_buffer(&self, src: &Buffer, dst: &Buffer, size: u64) {
        let region = vk::BufferCopy {
            size,
            src_offset: 0,
            dst_offset: 0,
        };

        unsafe {
            self.device
                .raw()
                .cmd_copy_buffer(self.command_buffer, src.raw(), dst.raw(), &[region]);
        }
    }

    pub(super) fn raw(&self) -> vk::CommandBuffer {
        self.command_buffer
    }
}

impl From<crate::TextureLayout> for vk::ImageLayout {
    fn from(value: crate::TextureLayout) -> Self {
        match value {
            crate::TextureLayout::Undefined => vk::ImageLayout::UNDEFINED,
            crate::TextureLayout::General => vk::ImageLayout::GENERAL,
            crate::TextureLayout::Color => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            crate::TextureLayout::DepthStencil => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            crate::TextureLayout::TransferSrc => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            crate::TextureLayout::TransferDst => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        }
    }
}
