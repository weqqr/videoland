use std::ffi::CStr;

use anyhow::Result;
use ash::vk::{self, PushConstantRange};

pub struct ShaderModule {
    shader_module: vk::ShaderModule,
}

impl ShaderModule {
    pub fn new(device: &ash::Device, spirv: &[u32]) -> Result<Self> {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(spirv);

        let shader_module = unsafe { device.create_shader_module(&create_info, None)? };

        Ok(Self { shader_module })
    }
}

pub struct PipelineDesc {
    pub vertex_shader: ShaderModule,
    pub fragment_shader: ShaderModule,
}

pub struct Pipeline {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl Pipeline {
    pub(super) fn new(device: &ash::Device, desc: &PipelineDesc) -> Result<Self> {
        let vertex_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .name(CStr::from_bytes_with_nul(b"vs_main\0").unwrap())
            .module(desc.vertex_shader.shader_module);

        let fragment_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .name(CStr::from_bytes_with_nul(b"fs_main\0").unwrap())
            .module(desc.fragment_shader.shader_module);

        let shader_stages = &[
            vertex_shader_stage_create_info.build(),
            fragment_shader_stage_create_info.build(),
        ];

        let vertex_attribute_descriptions = &[
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .offset(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .offset(3 * 4)
                .format(vk::Format::R32G32_SFLOAT)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .offset(3 * 4 + 2 * 4)
                .format(vk::Format::R32G32B32_SFLOAT)
                .build(),
        ];

        let vertex_binding_descriptions = &[vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(8 * 4)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(vertex_attribute_descriptions)
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

        let ranges = &[PushConstantRange::builder()
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

        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .input_assembly_state(&input_assembly_state)
            .vertex_input_state(&vertex_input_state)
            .multisample_state(&multisample_state)
            .layout(pipeline_layout)
            .rasterization_state(&rasterization_state)
            .stages(shader_stages)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state)
            .viewport_state(&viewport_state);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info.build()], None)
                .unwrap()
                .pop()
                .unwrap()
        };

        Ok(Pipeline {
            pipeline,
            pipeline_layout,
        })
    }
}
