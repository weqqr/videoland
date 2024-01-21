use std::ffi::CStr;
use std::sync::Arc;

use ash::vk;
use tinyvec::ArrayVec;

use crate::{Error, Device};

pub struct ShaderModule {
    device: Arc<Device>,
    shader_module: vk::ShaderModule,
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.raw().destroy_shader_module(self.shader_module, None);
        }
    }
}

impl ShaderModule {
    pub(super) unsafe fn new(device: Arc<Device>, spirv: &[u32]) -> Result<Self, Error> {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(spirv);

        let shader_module = device.raw().create_shader_module(&create_info, None)?;

        Ok(Self {
            device,
            shader_module,
        })
    }
}

pub struct Pipeline {
    device: Arc<Device>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.raw().destroy_pipeline(self.pipeline, None);
            self.device.raw().destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

impl Pipeline {
    const MAX_VERTEX_BUFFER_ATTRIBUTES: usize = 16;

    pub(super) unsafe fn new(device: Arc<Device>, desc: &crate::PipelineDesc) -> Result<Self, Error> {
        let vertex_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .name(CStr::from_bytes_with_nul(b"vs_main\0").unwrap())
            .module(desc.vertex.shader_module);

        let fragment_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .name(CStr::from_bytes_with_nul(b"fs_main\0").unwrap())
            .module(desc.fragment.shader_module);

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
            .stride(desc.vertex_layout.stride)
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
            .alpha_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .blend_enable(true);

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
            unsafe { device.raw().create_pipeline_layout(&pipeline_layout_create_info, None)? };

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

        let pipeline = device.raw()
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

    pub(super) fn raw(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub(super) fn raw_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
}

impl From<crate::VertexFormat> for vk::Format {
    fn from(value: crate::VertexFormat) -> Self {
        match value {
            crate::VertexFormat::Uint32x1 => vk::Format::R32_UINT,
            crate::VertexFormat::Float32x1 => vk::Format::R32_SFLOAT,
            crate::VertexFormat::Float32x2 => vk::Format::R32G32_SFLOAT,
            crate::VertexFormat::Float32x3 => vk::Format::R32G32B32_SFLOAT,
            crate::VertexFormat::Float32x4 => vk::Format::R32G32B32A32_SFLOAT,
        }
    }
}

impl From<crate::VertexAttribute> for vk::VertexInputAttributeDescription {
    fn from(value: crate::VertexAttribute) -> Self {
        vk::VertexInputAttributeDescription::builder()
            .binding(value.binding)
            .location(value.location)
            .offset(value.offset)
            .format(value.format.into())
            .build()
    }
}
