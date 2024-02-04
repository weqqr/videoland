use std::sync::Arc;

use ash::vk;

use crate::{Device, Error};

const BINDLESS_DESCRIPTOR_SET_SIZE: u32 = 1024;

pub struct BindlessDescriptorSet {
    device: Arc<Device>,

    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
    set: vk::DescriptorSet,
}

impl Drop for BindlessDescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.device
                .raw()
                .destroy_descriptor_set_layout(self.layout, None);
            self.device.raw().destroy_descriptor_pool(self.pool, None);
        }
    }
}

impl BindlessDescriptorSet {
    pub(super) unsafe fn new(device: Arc<Device>) -> Result<Self, Error> {
        let types = &[
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        ];

        let sizes = types
            .iter()
            .map(|ty| {
                vk::DescriptorPoolSize::builder()
                    .descriptor_count(1024)
                    .ty(*ty)
                    .build()
            })
            .collect::<Vec<_>>();

        let pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .max_sets(sizes.len() as u32 * BINDLESS_DESCRIPTOR_SET_SIZE)
            .pool_sizes(&sizes);

        let pool = device
            .raw()
            .create_descriptor_pool(&pool_create_info, None)?;

        // create descriptor set
        let flags = types
            .iter()
            .map(|_| {
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            })
            .collect::<Vec<_>>();

        let bindings = types
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(i as u32)
                    .descriptor_count(BINDLESS_DESCRIPTOR_SET_SIZE)
                    .descriptor_type(*ty)
                    .stage_flags(vk::ShaderStageFlags::ALL)
                    .build()
            })
            .collect::<Vec<_>>();

        let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&flags)
            .build();

        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .push_next(&mut binding_flags);

        let layout = device
            .raw()
            .create_descriptor_set_layout(&create_info, None)?;

        let allocate_layouts = &[layout];
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(allocate_layouts);

        let set = device.raw().allocate_descriptor_sets(&allocate_info)?[0];

        Ok(Self {
            device,
            pool,
            layout,
            set,
        })
    }
}
